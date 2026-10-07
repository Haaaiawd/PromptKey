// Module declarations
pub mod config;
pub mod context;
pub mod db;
pub mod hotkey;
pub mod injector;
pub mod ipc;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Engine lifecycle observable by the GUI (`check_hotkeys`, service dot).
/// `run_service` is invoked once per engine (re)start inside a fresh thread.
#[derive(Debug, Clone)]
pub enum EngineState {
    /// Not running — never started, stopped cleanly, or died before init.
    Stopped,
    /// Spawned, still initialising (config/db/injector/hotkey/pipe).
    Starting,
    /// Reached the main loop.
    Running,
    /// The engine thread panicked; carries the panic message.
    Failed(String),
}

static ENGINE_STATE: Mutex<EngineState> = Mutex::new(EngineState::Stopped);

pub fn set_engine_state(s: EngineState) {
    if let Ok(mut g) = ENGINE_STATE.lock() {
        *g = s;
    }
}

pub fn engine_state() -> EngineState {
    ENGINE_STATE
        .lock()
        .map(|g| g.clone())
        .unwrap_or(EngineState::Stopped)
}

/// `stop` is the cooperative shutdown flag (review F01/F28/F50/F64).
/// The loop exits promptly when set, then hotkeys/pipe workers are torn down
/// so a restarted engine never duplicates registrations or listeners.
pub fn run_service(stop: Arc<AtomicBool>) {
    // 初始化日志 — idempotent: run_service runs again on EVERY engine restart
    // (apply_settings / restart_service). The previous `init_from_env` panicked
    // on the second call (logger is process-global), silently killing the
    // restarted engine before hotkeys were ever registered — the toast still
    // said "saved". try_init returns Err instead of panicking.
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .try_init();
    set_engine_state(EngineState::Starting);
    println!("🔥 [INTERNAL_ENGINE] 提示词引擎正在子线程启动...");

    // 1. 初始化配置 (Moved up to get DB path)
    let config = crate::config::Config::load().unwrap_or_default();
    let hotkey_str = config.hotkey.clone();
    let quick_hotkey_str = config.quick_hotkey.clone();

    // 2. 初始化数据库
    let database = db::Database::new(&config.database_path).expect("无法初始化数据库");

    // 3. 初始化注入器 (platform trait object — Task8)
    let injector = injector::create(config.clone());

    // 3. 初始化上下文管理器 (platform trait object)
    let context_manager = context::create();

    // 5. 初始化热键服务 (platform trait object)
    let mut hotkey_service = hotkey::create(hotkey_str, quick_hotkey_str);
    if let Err(e) = hotkey_service.start() {
        log::error!("无法启动热键服务: {}", e);
    }

    // 6. 初始化 IPC 客户端 (用于通知 GUI 显示窗口)
    let ipc_client = ipc::IPCClient::default();

    // 7. 初始化逻辑注入服务端 (接收来自 GUI 的直接注入请求)
    let inject_rx = crate::ipc::inject_server::start(stop.clone());

    // 8. 进入主循环
    println!("✅ [INTERNAL_ENGINE] 引擎就绪，等待指令...");
    set_engine_state(EngineState::Running);

    // Store the context (window) that was active before opening the wheel/selector
    let mut last_active_context: Option<context::AppContext> = None;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        // A. 检查来自 GUI 的点选注入请求
        while let Ok(req) = inject_rx.try_recv() {
            println!("🎯 [ENGINE] 收到 GUI 注入请求: ID={}", req.prompt_id);
            // Use the captured context if available, otherwise try to get current (fallback)
            handle_injection_request(
                &database,
                injector.as_ref(),
                context_manager.as_ref(),
                Some(req.prompt_id),
                req.vars_json.as_deref(),
                last_active_context.as_ref(),
            );
        }

        // B. 检查热键事件
        while let Some(hotkey_id) = hotkey_service.try_wait_for_hotkey() {
            match hotkey_id {
                4 => {
                    println!("🎡 [HOTKEY] 触发提示词轮盘");
                    // Capture context before showing GUI
                    if let Ok(ctx) = context_manager.get_foreground_context() {
                        println!(
                            "💾 保存上下文: App={}, Title={}",
                            ctx.process_name, ctx.window_title
                        );
                        last_active_context = Some(ctx);
                    }
                    // Was `let _ =`: a dead pipe read exactly like "hotkey does
                    // nothing". The attempt is recorded in
                    // service::ipc::last_wheel_send() for diagnose_hotkey_pipeline.
                    if let Err(e) = ipc_client.send_show_wheel() {
                        eprintln!("❌ [HOTKEY] 轮盘唤起失败（pipe 发送失败）: {}", e);
                        log::error!("SHOW_WHEEL send failed: {}", e);
                    }
                }
                5 => {
                    // Phase 2 N4: quick hotkey → inject the configured default prompt
                    println!("⚡ [HOTKEY] 默认提示词直注");
                    let ctx = context_manager.get_foreground_context().ok();
                    handle_injection_request(
                        &database,
                        injector.as_ref(),
                        context_manager.as_ref(),
                        None,
                        None,
                        ctx.as_ref(),
                    );
                }
                _ => {}
            }
        }

        // 防止空转
        thread::sleep(Duration::from_millis(10));
    }

    // Shutdown: stop hotkey worker (unregisters hotkey ids) and drop the pipe
    // receiver; the inject server thread exits via its own stop watch.
    println!("🛑 [INTERNAL_ENGINE] 引擎停止");
    hotkey_service.stop();
    set_engine_state(EngineState::Stopped);
}

/// Phase 2 D1+D5: render a prompt's template content in a single pass.
/// - Custom {{var}} placeholders are filled from `vars_json` ({"name": "value"}).
/// - Automatic variables are always replaced: {{clipboard}}, {{date}}, {{time}}.
///   {{date}}/{{time}} are computed in UTC (local-time APIs are Task6 work).
/// Single-pass semantics (review F05/F20/F39/F66): supplied values are inserted
/// literally and never re-scanned, and automatic names are reserved — a
/// caller-supplied "clipboard"/"date"/"time" key cannot shadow the real values.
/// Whitespace inside braces is accepted ({{ name }} — F25/F35/F48/F69).
fn render_template(content: &str, vars_json: Option<&str>) -> String {
    let user_vars: serde_json::Map<String, serde_json::Value> = vars_json
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let mut clip: Option<String> = None;
    let mut now: Option<(String, String)> = None;
    let is_var_name = |s: &str| {
        let mut cs = s.chars();
        matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
            && cs.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
    };
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find("{{") {
        let (head, tail) = rest.split_at(start);
        out.push_str(head);
        let inner = &tail[2..];
        match inner.find("}}") {
            Some(end) if is_var_name(inner[..end].trim()) => {
                let name = inner[..end].trim();
                match name {
                    "clipboard" => {
                        let v = clip.get_or_insert_with(|| injector::clipboard_text().unwrap_or_default());
                        out.push_str(v.as_str());
                    }
                    "date" | "time" => {
                        let dt = now.get_or_insert_with(utc_now_strings);
                        out.push_str(if name == "date" { dt.0.as_str() } else { dt.1.as_str() });
                    }
                    _ => match user_vars.get(name).and_then(|v| v.as_str()) {
                        Some(v) => out.push_str(v),
                        None => out.push_str(&tail[..end + 4]), // unresolved stays literal
                    },
                }
                rest = &inner[end + 2..];
            }
            _ => {
                out.push_str("{{");
                rest = inner;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Current UTC as ("YYYY-MM-DD", "HH:MM") — no extra deps.
/// Date uses Howard Hinnant's civil-from-days algorithm.
fn utc_now_strings() -> (String, String) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (
        format!("{:04}-{:02}-{:02}", y, m, d),
        format!("{:02}:{:02}", rem / 3600, (rem % 3600) / 60),
    )
}

fn handle_injection_request(
    db: &db::Database,
    injector: &dyn injector::Injector,
    ctx: &dyn context::Context,
    force_id: Option<i32>,
    vars_json: Option<&str>,
    target_override: Option<&context::AppContext>,
) {
    // 1. 获取目标上下文
    // 如果有 override (来自轮盘/面板调用)，使用保存的上下文；否则获取当前上下文
    let context = if let Some(override_ctx) = target_override {
        log::info!("⚡ 使用保存的上下文: {}", override_ctx.window_title);
        override_ctx.clone()
    } else {
        ctx.get_foreground_context()
            .unwrap_or(crate::context::AppContext {
                process_name: "Unknown".to_string(),
                window_title: "Unknown".to_string(),
                window_handle: 0,
            })
    };

    let app_name = context.process_name.clone();
    let window_title = context.window_title.clone();

    log::info!(
        "⚡ 处理注入请求 | App: {} | Title: {} | ForceID: {:?}",
        app_name,
        window_title,
        force_id
    );

    // 2. 确定要使用的 Prompt
    let prompt_result = if let Some(id) = force_id {
        // A. 强制指定模式 (来自 UI 选择)
        db.get_prompt_by_id(id).map(|p| (p, "wheel_select"))
    } else {
        // B. 自动匹配模式 (来自快捷键)
        match db.find_prompt_for_context(&app_name, &window_title) {
            Ok(Some(p)) => Ok((p, "hotkey_inject")),
            Ok(None) => {
                println!("⚠️ 当前上下文没有匹配的提示词");
                return;
            }
            Err(e) => Err(e),
        }
    };

    // 3. 执行注入
    match prompt_result {
        Ok((prompt, action_type)) => {
            let rendered = render_template(&prompt.content, vars_json);
            println!("✨ 正在注入: [{}]", prompt.name);

            // 构造注入上下文
            let injection_ctx = injector::InjectionContext {
                app_name: app_name.clone(),
                window_title: window_title.clone(),
                window_handle: context.window_handle,
            };

            // 调用注入器，然后按真实结果写日志（Task6：不再硬编码 success=true）
            match injector.inject(&rendered, &injection_ctx) {
                Ok((strategy, elapsed)) => {
                    println!("✅ 注入成功");
                    if let Err(e) = db.log_usage(
                        prompt.id,
                        &prompt.name,
                        &app_name,
                        &window_title,
                        "Internal",
                        &strategy,
                        elapsed as u128,
                        true,
                        None,
                        "Injected",
                        action_type,
                    ) {
                        log::error!("无法记录使用日志: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("❌ 注入失败: {}", e);
                    println!("❌ 注入失败: {}", e);
                    let err_msg = e.to_string();
                    if let Err(le) = db.log_usage(
                        prompt.id,
                        &prompt.name,
                        &app_name,
                        &window_title,
                        "Internal",
                        "Failed",
                        0,
                        false,
                        Some(&err_msg),
                        "Failed",
                        action_type,
                    ) {
                        log::error!("无法记录失败日志: {}", le);
                    }
                }
            }
        }
        Err(e) => {
            log::error!("查询提示词失败: {}", e);
        }
    }
}

// 为了作为二进制文件运行时兼容
#[allow(dead_code)]
fn main() {
    run_service(Arc::new(AtomicBool::new(false)));
}
