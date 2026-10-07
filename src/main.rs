// Task8: gate the Windows GUI subsystem so the crate builds on other platforms
#![cfg_attr(windows, windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder, AppHandle, Emitter,
};
use tauri_plugin_dialog::DialogExt;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// 服务进程句柄
#[cfg(windows)]
mod ipc_listener; // named pipes (tokio::net::windows) are Windows-only
mod inject_pipe_client; // TW004: GUI → Service injection command client


struct ServiceState {
    is_active: bool,
    shutdown: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

// 提示词结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Prompt {
    id: Option<i32>,
    name: String,
    tags: Option<Vec<String>>,
    content: String,
    content_type: Option<String>,
    variables_json: Option<String>,
    app_scopes_json: Option<String>,
    inject_order: Option<String>,
    version: Option<i32>,
    updated_at: Option<String>,
}

// Phase 2: unified prompt view row (prompts page + wheel both consume this)
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PromptView {
    id: i32,
    name: String,
    content: String,
    tags: Vec<String>,
    content_type: Option<String>,
    variables_json: Option<String>,
    app_scopes_json: Option<String>,
    inject_order: Option<String>,
    version: Option<i32>,
    updated_at: Option<String>,
    is_pinned: bool,
    usage_count: i64,
    last_used_at: Option<i64>,
    frecency: f64,
}


impl ServiceState {
    fn new() -> Self {
        ServiceState { is_active: false, shutdown: None, worker: None }
    }
    
    fn is_running(&mut self) -> bool {
        // Honest check — a panicked engine left is_active=true while dead.
        if self.is_active {
            let dead = self.worker.as_ref().map(|w| w.is_finished()).unwrap_or(true);
            if dead {
                self.is_active = false;
            }
        }
        self.is_active
    }

    fn start_service(&mut self) -> Result<(), String> {
        if self.is_active {
            println!("✅ 内嵌服务已在运行中");
            return Ok(());
        }

        println!("🚀 正在启动内嵌提示词引擎 (Embedded Thread)...");

        // 启动后台线程运行 Service 逻辑；stop 标志位让引擎循环可退出
        service::set_engine_state(service::EngineState::Starting);
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_thread = stop.clone();
        let worker = std::thread::spawn(move || {
            // A panicking engine must not vanish silently — record it so
            // check_hotkeys / apply_settings can surface a real error.
            // (release builds abort on panic regardless; catch_unwind covers
            // unwind profiles and any future change.)
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                service::run_service(stop_thread);
            }));
            if let Err(payload) = res {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                service::set_engine_state(service::EngineState::Failed(msg));
            }
        });

        self.shutdown = Some(stop);
        self.worker = Some(worker);
        // 设置为已激活
        self.is_active = true;

        // Bounded wait for the engine to prove itself: a crash during init
        // (previously: env_logger re-init panic; still possible: db init) must
        // come back as an error, not a green "started".
        for _ in 0..200 {
            match service::engine_state() {
                service::EngineState::Running => break,
                service::EngineState::Failed(e) => {
                    self.is_active = false;
                    return Err(format!("引擎启动失败: {}", e));
                }
                _ => {
                    if self.worker.as_ref().map(|w| w.is_finished()).unwrap_or(true) {
                        self.is_active = false;
                        return Err("引擎启动失败: 线程异常退出".to_string());
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        Ok(())
    }
    
    fn stop_service(&mut self) -> Result<(), String> {
        println!("🛑 正在停止内嵌提示词引擎...");
        // Review F01/F28/F50/F64: actually terminate the engine — signal the
        // loop, join the thread (it tears down hotkey + pipe workers), so a
        // restart never leaves competing engines alive.
        if let Some(stop) = self.shutdown.take() {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.is_active = false;
        Ok(())
    }
}

fn main() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());
    
    // 为桌面平台添加单实例插件
    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            println!("检测到新实例启动，聚焦到现有窗口");
            
            // 尝试显示和聚焦主窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.unminimize();
            } else {
                // 如果主窗口不存在，创建并显示它
                create_and_show_window(app);
            }
        }));
    }
    
    builder
        .manage(Mutex::new(ServiceState::new()))
        // 关闭窗口：直接隐藏到托盘（避免反复触发 CloseRequested 导致“点击无效”）
        .on_window_event(|app, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_service,
            stop_service,
            restart_service,
            check_service_status,
            check_hotkeys,
            apply_settings,
            get_settings,
            get_all_prompts,
            get_prompts_view,
            get_app_settings,
            set_app_setting,
            set_pin_order,
            trigger_wheel_injection,       // TW005: PromptWheel injection trigger
            trigger_wheel_injection_vars,  // Phase2: inject with {{var}} values
            show_wheel_window,             // TW012: Show PromptWheel window (cursor-following)
            diagnose_hotkey_pipeline,      // P0 fix: locate where the hotkey→wheel chain breaks
            present_main_window_new_prompt, // D4 fix: wheel quick-create hands off to the main editor
            create_prompt,
            update_prompt,
            delete_prompt,
            reset_settings,
            set_selected_prompt,
            get_selected_prompt,
            get_usage_logs,
            exit_application,
            clear_usage_logs,
            toggle_prompt_pin,
            export_prompts_pack,
            import_prompts_pack,
            fetch_pack_url,
            pick_pack_file
        ])
        .setup(|app| {
            // 创建系统托盘菜单
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "显示/隐藏", true, None::<&str>)?;
            
            // T1-010: Start IPC Listener (named pipe → Windows-only)
            #[cfg(windows)]
            ipc_listener::start_ipc_listener(app.handle().clone());
            
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            
            // 创建系统托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        // 退出前先尝试停止服务
                        if let Ok(service_state) = app.state::<Mutex<ServiceState>>().lock() {
                            let mut ss = service_state;
                            let _ = ss.stop_service();
                        }
                        app.exit(0);
                    }
                    "show" => {
                        toggle_window_visibility(app);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::DoubleClick { .. } = event {
                        toggle_window_visibility(tray.app_handle());
                    }
                })
                .build(app)?;
            
            // TW012: Pre-create PromptWheel window (hidden state)
            // Phase 2 Wheel A: 280px wheel + petal overflow + shadow = 320px window
            let wheel_window = WebviewWindowBuilder::new(
                app,
                "wheel-panel",
                WebviewUrl::App("wheel.html".into())
            )
            .title("PromptWheel")
            .inner_size(320.0, 320.0)
            .resizable(false)
            .decorations(false)       // Borderless
            .transparent(true)        // Transparent background (Crucial for Donut shape)
            .shadow(false)            // CRITICAL: Connects to transparent? No, this removes the native window shadow artifact!
            .always_on_top(true)      // Always on top
            .skip_taskbar(true)       // Don't show in taskbar
            .visible(false)           // Start hidden
            .build()?;
            
            // TW014: Register focus lost event to auto-hide wheel.
            // Grace window: Windows can grant then immediately retract
            // foreground after a hotkey-driven SetForegroundWindow (focus
            // bounce). A blur inside PRESENT_BLUR_GRACE of the present call is
            // not the user clicking away — hiding then is what made the wheel
            // appear to "never open".
            let wheel_window_clone = wheel_window.clone();
            wheel_window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let bounced = LAST_PRESENT_AT
                        .lock()
                        .ok()
                        .and_then(|g| *g)
                        .map(|t| t.elapsed() < PRESENT_BLUR_GRACE)
                        .unwrap_or(false);
                    if !bounced {
                        let _ = wheel_window_clone.hide();
                    }
                }
            });
            
            println!("✅ PromptWheel window pre-created (hidden)");
            
            // 启动时自动创建并显示窗口
            create_and_show_window(&app.handle());
            
            // 启动服务
            let service_state = app.state::<Mutex<ServiceState>>();
            let mut service_state = service_state.lock().unwrap();
            if let Err(e) = service_state.start_service() {
                eprintln!("启动服务时出错: {}", e);
            } else {
                println!("服务启动成功");
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn start_service(app: AppHandle) -> Result<String, String> {
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    match service_state.start_service() {
        Ok(()) => Ok("服务启动成功".to_string()),
        Err(e) => Err(e)
    }
}

#[tauri::command]
fn stop_service(app: AppHandle) -> Result<String, String> {
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    match service_state.stop_service() {
        Ok(()) => Ok("服务停止成功".to_string()),
        Err(e) => Err(e)
    }
}

#[tauri::command]
fn check_service_status(app: AppHandle) -> Result<bool, String> {
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    Ok(service_state.is_running())
}

#[tauri::command]
fn get_all_prompts() -> Result<Vec<Prompt>, String> {
    // 连接数据库（确保目录与表存在）
    let conn = open_db()?;
    
    // 查询所有提示词
    let mut stmt = conn.prepare(
        "SELECT id, name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version, updated_at
         FROM prompts"
    ).map_err(|e| format!("无法准备查询语句: {}", e))?;
    
    let prompt_iter = stmt.query_map([], |row| {
        // 反序列化tags字段
        let tags_str: Option<String> = row.get(2).map_err(|e| rusqlite::Error::from(e))?;
        let tags = match tags_str {
            Some(s) => {
                match serde_json::from_str(&s) {
                    Ok(tags) => Some(tags),
                    Err(_) => None,
                }
            }
            None => None,
        };
        
        Ok(Prompt {
            id: row.get(0).map_err(|e| rusqlite::Error::from(e))?,
            name: row.get(1).map_err(|e| rusqlite::Error::from(e))?,
            tags,
            content: row.get(3).map_err(|e| rusqlite::Error::from(e))?,
            content_type: row.get(4).map_err(|e| rusqlite::Error::from(e))?,
            variables_json: row.get(5).map_err(|e| rusqlite::Error::from(e))?,
            app_scopes_json: row.get(6).map_err(|e| rusqlite::Error::from(e))?,
            inject_order: row.get(7).map_err(|e| rusqlite::Error::from(e))?,
            version: row.get(8).map_err(|e| rusqlite::Error::from(e))?,
            updated_at: row.get(9).map_err(|e| rusqlite::Error::from(e))?,
        })
    }).map_err(|e| format!("查询失败: {}", e))?;
    
    let mut prompts = Vec::new();
    for prompt in prompt_iter {
        prompts.push(prompt.map_err(|e| format!("获取提示词失败: {}", e))?);
    }
    
    Ok(prompts)
}

// Phase 2: one query serving both the prompts grid and the wheel.
// usage_count / last_used_at only count successful injections; frecency =
// successes with a 7-day half-life style recency denominator.
#[tauri::command]
fn get_prompts_view() -> Result<Vec<PromptView>, String> {
    let conn = open_db()?;

    let mut stmt = conn.prepare(
        "SELECT
            p.id, p.name, p.content, p.tags, p.content_type, p.variables_json,
            p.app_scopes_json, p.inject_order, p.version, p.updated_at,
            COALESCE(p.is_pinned, 0) AS is_pinned,
            COALESCE(s.usage_count, 0) AS usage_count,
            s.last_used_s AS last_used_at_ms,
            COALESCE(
                (COALESCE(s.usage_count, 0) * 1.0)
                / (1.0 + COALESCE(julianday('now') - julianday(
                    COALESCE(s.last_used, p.updated_at)), 0.0) / 7.0),
                0.0) AS frecency
         FROM prompts p
         LEFT JOIN (
            SELECT prompt_id,
                   COUNT(*) AS usage_count,
                   MAX(created_at) AS last_used,
                   MAX(strftime('%s', created_at)) * 1000 AS last_used_s
            FROM usage_logs
            WHERE success = 1
            GROUP BY prompt_id
         ) s ON s.prompt_id = p.id
         ORDER BY is_pinned DESC, frecency DESC, p.id DESC"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt.query_map([], |row| {
        let tags_str: Option<String> = row.get(3)?;
        let tags: Vec<String> = tags_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Ok(PromptView {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
            tags,
            content_type: row.get(4)?,
            variables_json: row.get(5)?,
            app_scopes_json: row.get(6)?,
            inject_order: row.get(7)?,
            version: row.get(8)?,
            updated_at: row.get(9)?,
            is_pinned: row.get::<_, i32>(10)? == 1,
            usage_count: row.get(11)?,
            last_used_at: row.get::<_, Option<i64>>(12)?,
            frecency: row.get(13)?,
        })
    }).map_err(|e| format!("Query failed: {}", e))?;

    let mut prompts = Vec::new();
    for p in rows {
        prompts.push(p.map_err(|e| format!("Failed to fetch prompt: {}", e))?);
    }
    Ok(prompts)
}

// Phase 2: key/value app settings table (UI prefs + default prompt policy)
#[tauri::command]
fn get_app_settings() -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_settings")
        .map_err(|e| format!("Failed to read app_settings: {}", e))?;
    let mut map = serde_json::Map::new();
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| format!("Query failed: {}", e))?;
    for r in rows {
        let (k, v) = r.map_err(|e| format!("Read failed: {}", e))?;
        map.insert(k, serde_json::Value::String(v));
    }
    Ok(map)
}

#[tauri::command]
fn set_app_setting(key: String, value: String) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    ).map_err(|e| format!("Failed to write setting: {}", e))?;
    Ok(())
}

// Phase 2 D2: persist manual pin order (inject_order column)
#[tauri::command]
fn set_pin_order(ids: Vec<i32>) -> Result<(), String> {
    let conn = open_db()?;
    for (i, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE prompts SET inject_order = ?1 WHERE id = ?2",
            rusqlite::params![(i + 1).to_string(), id],
        ).map_err(|e| format!("Failed to set order: {}", e))?;
    }
    Ok(())
}

// TW005: Trigger wheel injection command
// Called by wheel UI when user selects a prompt
#[tauri::command]
fn trigger_wheel_injection(prompt_id: i32) -> Result<(), String> {
    inject_pipe_client::send_inject_request(prompt_id)
        .map_err(|e| format!("Failed to send inject request: {}", e))
}

// Phase 2 D5: inject a prompt after the wheel collected {{var}} values
#[tauri::command]
fn trigger_wheel_injection_vars(prompt_id: i32, vars_json: String) -> Result<(), String> {
    inject_pipe_client::send_inject_request_vars(prompt_id, vars_json)
        .map_err(|e| format!("Failed to send inject request: {}", e))
}

/// Instant of the most recent `present_wheel` — read by the Focused(false)
/// autohide to ignore focus-bounce right after a hotkey-driven summon.
static LAST_PRESENT_AT: Mutex<Option<std::time::Instant>> = Mutex::new(None);
/// Focus events arriving within this window of a present are treated as
/// bounce, not as "user clicked away".
const PRESENT_BLUR_GRACE: Duration = Duration::from_millis(700);

/// Last `present_wheel` outcome, surfaced by `diagnose_hotkey_pipeline`.
#[derive(Debug, Clone, Serialize)]
pub struct PresentRecord {
    /// Unix epoch ms.
    pub at_ms: u64,
    pub ok: bool,
    pub detail: Option<String>,
}
static LAST_PRESENT: Mutex<Option<PresentRecord>> = Mutex::new(None);

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// Position the wheel window centered on the mouse cursor, inside the monitor
// that contains the cursor, then make it visible and arm the JS side.
// Every call records into LAST_PRESENT — the pipe→present chain used to fail
// invisibly (a transparent 320px overlay shows literally nothing).
pub(crate) fn present_wheel(app: &AppHandle) -> Result<(), String> {
    let r = present_wheel_inner(app);
    if let Ok(mut g) = LAST_PRESENT.lock() {
        *g = Some(PresentRecord {
            at_ms: epoch_ms(),
            ok: r.is_ok(),
            detail: r.as_ref().err().cloned(),
        });
    }
    r
}

fn present_wheel_inner(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("wheel-panel") else {
        return Err("wheel window 'wheel-panel' does not exist".to_string());
    };
    // Review F03/F56/F71: monitors can have negative/virtual origins, so a
    // `.max(0.0)` floor teleported the wheel onto the primary display.
    // Clamp against the cursor monitor's position+size instead.
    if let Ok(pos) = app.cursor_position() {
        let mut x = pos.x - 160.0;
        let mut y = pos.y - 160.0;
        if let Ok(Some(mon)) = window.monitor_from_point(pos.x, pos.y) {
            let mp = mon.position();
            let ms = mon.size();
            let margin = 16.0;
            // clamp() panics on inverted ranges (monitor smaller than the window)
            let lo_x = mp.x as f64 + margin;
            let hi_x = (mp.x as f64 + ms.width as f64 - 320.0 - margin).max(lo_x);
            let lo_y = mp.y as f64 + margin;
            let hi_y = (mp.y as f64 + ms.height as f64 - 320.0 - margin).max(lo_y);
            x = x.clamp(lo_x, hi_x);
            y = y.clamp(lo_y, hi_y);
        }
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    // Emit BEFORE show(): if the webview's listener is gone the wheel can
    // never render, and a shown-but-unrendered transparent overlay is the
    // worst possible state (invisible click-catcher). Emit failure keeps the
    // window hidden and surfaces as Err.
    window
        .emit("wheel-show", ())
        .map_err(|e| format!("emit wheel-show failed: {}", e))?;
    // Arm the blur-bounce grace before the window can gain/lose focus.
    if let Ok(mut g) = LAST_PRESENT_AT.lock() {
        *g = Some(std::time::Instant::now());
    }
    window
        .show()
        .map_err(|e| format!("show wheel window failed: {}", e))?;
    // Best-effort: Windows may deny SetForegroundWindow to a process that was
    // not foreground. A denied grab is NOT fatal — the wheel stays usable —
    // but it used to abort before the emit, leaving the overlay invisible.
    if let Err(e) = window.set_focus() {
        eprintln!("[wheel] set_focus failed (non-fatal): {}", e);
    }
    println!("✅ Wheel window shown at cursor");
    Ok(())
}

/// Self-diagnostic for the hotkey→wheel chain (settings → 诊断按钮). Every
/// link reports what it actually saw so a silent break is locatable:
/// engine state → per-hotkey registration → pipe send record → GUI listener
/// record → wheel window presence. `hints` are machine-readable; the UI
/// localizes them.
#[tauri::command]
fn diagnose_hotkey_pipeline(app: AppHandle) -> Result<serde_json::Value, String> {
    let cfg = load_or_default_config()?;
    let want = [(4u32, cfg.hotkey.clone()), (5u32, cfg.quick_hotkey.clone())];
    let hotkeys = service::hotkey::check(&want);

    let (engine, engine_error) = match service::engine_state() {
        service::EngineState::Running => ("running", None),
        service::EngineState::Starting => ("starting", None),
        service::EngineState::Failed(e) => ("failed", Some(e)),
        service::EngineState::Stopped => ("stopped", None),
    };

    #[cfg(windows)]
    let (listener, listener_alive, listener_last_msg_at) = {
        let st = ipc_listener::listener_state();
        (
            serde_json::to_value(&st).unwrap_or_default(),
            st.alive,
            st.last_message_at_ms,
        )
    };
    #[cfg(not(windows))]
    let (listener, listener_alive, listener_last_msg_at) = (
        serde_json::json!({ "alive": false, "platform": "named pipes are Windows-only" }),
        false,
        None,
    );

    let last_send = service::ipc::last_wheel_send();
    let last_send_json = serde_json::to_value(&last_send).unwrap_or_default();
    let last_present = LAST_PRESENT
        .lock()
        .ok()
        .and_then(|g| g.clone());
    let last_present_json = serde_json::to_value(&last_present).unwrap_or_default();

    let wheel = app.get_webview_window("wheel-panel");
    let (wheel_exists, wheel_visible, wheel_focused) = match &wheel {
        Some(w) => (
            true,
            w.is_visible().unwrap_or(false),
            w.is_focused().unwrap_or(false),
        ),
        None => (false, false, false),
    };

    let mut hints: Vec<String> = Vec::new();
    if engine != "running" {
        hints.push("engine_not_running".into());
    }
    #[cfg(windows)]
    {
        if !listener_alive {
            hints.push("listener_not_running".into());
        }
    }
    #[cfg(not(windows))]
    {
        let _ = listener_last_msg_at;
        hints.push("unsupported_platform".into());
    }
    if let Some(s) = &last_send {
        if s.kind == "open_failed" || s.kind == "write_failed" {
            hints.push("send_failed".into());
        } else if s.kind == "sent" {
            // Sent but never logged by the listener → the break is inside the
            // pipe itself (or a different process owns the name).
            let received = listener_last_msg_at
                .map(|t| t >= s.at_ms)
                .unwrap_or(false);
            if !received {
                hints.push("send_no_receipt".into());
            }
        }
    }
    if let Some(p) = &last_present {
        if !p.ok {
            hints.push("present_failed".into());
        }
    }
    if !wheel_exists {
        hints.push("wheel_window_missing".into());
    }
    if engine == "running"
        && hotkeys
            .iter()
            .any(|h| h.status != "ok" && h.status != "disabled")
    {
        hints.push("hotkey_not_ok".into());
    }
    if hints.is_empty() {
        hints.push("ok".into());
    }

    Ok(serde_json::json!({
        "engine": engine,
        "engine_error": engine_error,
        "hotkeys": hotkeys,
        "pipe": {
            "listener": listener,
            "listener_alive": listener_alive,
            "last_send": last_send_json,
            "last_present": last_present_json,
        },
        "wheel_window": {
            "exists": wheel_exists,
            "visible": wheel_visible,
            "focused": wheel_focused,
        },
        "hints": hints,
    }))
}

// TW012: Show wheel window command (also used by the in-app wheel preview button)
#[tauri::command]
fn show_wheel_window(app: AppHandle) -> Result<(), String> {
    present_wheel(&app)
}

// Phase 2 D4 fix: the wheel's quick-create gesture cannot author a prompt body
// inside a transient 320px overlay, so it hands the typed filter text to the
// main window's editor instead of fabricating a record. The drawer enforces
// non-empty content, so nothing half-baked can reach the wheel.
#[tauri::command]
fn present_main_window_new_prompt(app: AppHandle, name: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        // Stage the draft first — even if presenting the window fails, the
        // seeded drawer is waiting the next time it is shown.
        window
            .emit("wheel-new-prompt", name)
            .map_err(|e| format!("Emit failed: {}", e))?;
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

// Wheel: Toggle prompt pin status
#[tauri::command]
fn toggle_prompt_pin(id: i32) -> Result<bool, String> {
    let conn = open_db()?;

    // Get current pin status
    let current_pin: i32 = conn
        .query_row("SELECT COALESCE(is_pinned, 0) FROM prompts WHERE id = ?1", [id], |row| row.get(0))
        .map_err(|e| format!("Failed to get pin status: {}", e))?;

    // Toggle
    let new_pin = if current_pin == 0 { 1 } else { 0 };

    conn.execute("UPDATE prompts SET is_pinned = ?1 WHERE id = ?2", [new_pin, id])
        .map_err(|e| format!("Failed to update pin status: {}", e))?;

    Ok(new_pin == 1)
}

/* ---------- Phase 2: promptkey-pack import/export (N5) ---------- */

fn pack_json_from_db() -> Result<serde_json::Value, String> {
    let conn = open_db()?;
    // Review F34: include variables_json + content_type so a pack round-trip
    // preserves every prompt property.
    let mut stmt = conn.prepare(
        "SELECT name, content, tags, app_scopes_json, inject_order, COALESCE(is_pinned,0),
                content_type, variables_json
         FROM prompts ORDER BY id ASC"
    ).map_err(|e| format!("Failed to read prompts: {}", e))?;
    let rows = stmt.query_map([], |row| {
        let tags_str: Option<String> = row.get(2)?;
        let tags: Vec<String> = tags_str.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let apps_str: Option<String> = row.get(3)?;
        let app_scopes: Vec<String> = apps_str.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Ok(serde_json::json!({
            "name": row.get::<_, String>(0)?,
            "content": row.get::<_, String>(1)?,
            "tags": tags,
            "app_scopes": app_scopes,
            "inject_order": row.get::<_, Option<String>>(4)?,
            "pinned": row.get::<_, i32>(5)? == 1,
            "content_type": row.get::<_, Option<String>>(6)?,
            "variables_json": row.get::<_, Option<String>>(7)?,
        }))
    }).map_err(|e| format!("Query failed: {}", e))?;
    let mut prompts = Vec::new();
    for r in rows { prompts.push(r.map_err(|e| format!("Row failed: {}", e))?); }
    Ok(serde_json::json!({
        "format": "promptkey-pack",
        "version": 1,
        "pack": { "name": "PromptKey Export", "author": "user", "lang": "mixed" },
        "prompts": prompts,
    }))
}

fn import_pack_obj(obj: &serde_json::Value) -> Result<(usize, usize), String> {
    if obj.get("format").and_then(|f| f.as_str()) != Some("promptkey-pack") {
        return Err("Not a promptkey-pack file".into());
    }
    let prompts = obj.get("prompts").and_then(|p| p.as_array())
        .ok_or_else(|| "Pack has no prompts array".to_string())?;
    let conn = open_db()?;
    let mut added = 0usize;
    let mut skipped = 0usize;
    for p in prompts {
        let name = match p.get("name").and_then(|v| v.as_str()) { Some(n) if !n.trim().is_empty() => n.trim(), _ => continue };
        let content = match p.get("content").and_then(|v| v.as_str()) { Some(c) if !c.is_empty() => c, _ => continue };
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM prompts WHERE name = ?1", [name], |r| r.get(0)
        ).unwrap_or(0);
        if exists > 0 { skipped += 1; continue; }
        let tags: Vec<String> = p.get("tags").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        let apps: Vec<String> = p.get("app_scopes").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        let pinned = if p.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 };
        let order = p.get("inject_order").and_then(|v| v.as_str().map(|s| s.to_string()));
        // Review F34: round-trip content_type + variables_json too.
        let content_type = p.get("content_type").and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "text".to_string());
        let variables_json = p.get("variables_json").and_then(|v| v.as_str().map(|s| s.to_string()));
        conn.execute(
            "INSERT INTO prompts (name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version, is_pinned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)",
            rusqlite::params![
                name,
                serde_json::to_string(&tags).unwrap_or_default(),
                content,
                content_type,
                variables_json,
                serde_json::to_string(&apps).unwrap_or_default(),
                order,
                pinned,
            ],
        ).map_err(|e| format!("Insert failed: {}", e))?;
        added += 1;
    }
    Ok((added, skipped))
}

#[tauri::command]
async fn export_prompts_pack(app: AppHandle) -> Result<Option<String>, String> {
    let pack = pack_json_from_db()?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name("promptkey-pack.json")
        .add_filter("PromptKey Pack", &["json"])
        .save_file(move |path| { let _ = tx.send(path); });
    let picked = rx.await.map_err(|e| format!("Dialog failed: {}", e))?;
    let Some(fp) = picked else { return Ok(None) };
    let path = fp.as_path().ok_or_else(|| "Invalid path".to_string())?;
    std::fs::write(&path, serde_json::to_string_pretty(&pack).unwrap_or_default())
        .map_err(|e| format!("Write failed: {}", e))?;
    Ok(Some(path.display().to_string()))
}

#[derive(Serialize)]
struct ImportResult { added: usize, skipped: usize }

#[tauri::command]
async fn import_prompts_pack(app: AppHandle) -> Result<Option<ImportResult>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("PromptKey Pack", &["json"])
        .pick_file(move |path| { let _ = tx.send(path); });
    let picked = rx.await.map_err(|e| format!("Dialog failed: {}", e))?;
    let Some(fp) = picked else { return Ok(None) };
    let path = fp.as_path().ok_or_else(|| "Invalid path".to_string())?;
    // Review F32: cap file size before reading — no preview happens until
    // parse, and a multi-GB file would exhaust memory first.
    let size = std::fs::metadata(&path).map_err(|e| format!("Stat failed: {}", e))?.len();
    if size > MAX_PACK_FILE_BYTES {
        return Err(format!("Pack file too large ({} bytes > {})", size, MAX_PACK_FILE_BYTES));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Read failed: {}", e))?;
    let obj: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("Parse failed: {}", e))?;
    let (added, skipped) = import_pack_obj(&obj)?;
    Ok(Some(ImportResult { added, skipped }))
}

/* ---------- pack URL fetch with SSRF protection (review F04/F19/F30/F43/F53/F65, F31/F54) ---------- */

const MAX_PACK_BYTES: u64 = 1024 * 1024;
const MAX_PACK_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PACK_REDIRECTS: u32 = 3;

/// Publicly routable destination? Rejects loopback, private, link-local
/// (incl. cloud metadata 169.254.169.254), CGNAT, reserved and doc ranges.
fn ip_is_public(ip: &std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_private() || v4.is_loopback() || v4.is_link_local()
                || v4.is_unspecified() || v4.is_multicast() || v4.is_broadcast()
                || o[0] == 0                                       // 0.0.0.0/8
                || (o[0] == 100 && (o[1] & 0xC0) == 64)            // CGNAT 100.64/10
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)         // 192.0.0.0/24
                || (o[0] == 192 && o[1] == 0 && o[2] == 2)         // TEST-NET-1
                || (o[0] == 198 && (o[1] == 18 || o[1] == 19))     // benchmarking
                || (o[0] == 198 && o[1] == 51 && o[2] == 100)      // TEST-NET-2
                || (o[0] == 203 && o[1] == 0 && o[2] == 113)       // TEST-NET-3
                || o[0] >= 240)                                    // reserved/broadcast
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return ip_is_public(&IpAddr::V4(mapped));
            }
            !(v6.is_loopback() || v6.is_unspecified() || v6.is_multicast()
                || v6.is_unicast_link_local() || v6.is_unique_local())
        }
    }
}

fn validate_pack_url(raw: &str) -> Result<url::Url, String> {
    let u = url::Url::parse(raw).map_err(|e| format!("Invalid URL: {}", e))?;
    if u.scheme() != "https" {
        return Err("Only https:// pack URLs are allowed".into());
    }
    if u.host_str().is_none() {
        return Err("URL has no host".into());
    }
    Ok(u)
}

/// DNS resolver that refuses non-public destinations. Returning the validated
/// addresses also pins the DNS answer for the actual connect, which closes
/// the rebind window between "check IP" and "connect" (review SSRF fix).
fn public_resolver() -> impl Fn(&str) -> std::io::Result<Vec<std::net::SocketAddr>> + Send + Sync {
    |addr: &str| {
        use std::net::ToSocketAddrs;
        let resolved: Vec<std::net::SocketAddr> = addr.to_socket_addrs()?.collect();
        let public: Vec<_> = resolved.into_iter().filter(|a| ip_is_public(&a.ip())).collect();
        if public.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "pack URL resolves to a non-public address",
            ));
        }
        Ok(public)
    }
}

// Task5: fetch a pack from a URL (explicit user action; response capped at 1MB)
#[tauri::command]
async fn fetch_pack_url(url: String) -> Result<String, String> {
    validate_pack_url(&url)?;
    let text = tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        let agent = ureq::AgentBuilder::new()
            .redirects(0)           // manual: every hop is re-validated below
            .https_only(true)       // also refuses https -> http downgrades
            .timeout(std::time::Duration::from_secs(15))
            .resolver(public_resolver())
            .build();
        let mut current = url;
        for _ in 0..=MAX_PACK_REDIRECTS {
            let resp = agent.get(&current).call()
                .map_err(|e| format!("HTTP error: {}", e))?;
            if (300..400).contains(&resp.status()) {
                let loc = resp.header("Location").ok_or("Redirect without Location")?;
                let next = url::Url::parse(&current)
                    .and_then(|b| b.join(loc))
                    .map_err(|e| format!("Bad redirect target: {}", e))?;
                validate_pack_url(next.as_str())?;
                current = next.to_string();
                continue;
            }
            let mut buf = String::new();
            resp.into_reader()
                .take(MAX_PACK_BYTES)
                .read_to_string(&mut buf)
                .map_err(|e| format!("Read error: {}", e))?;
            return Ok::<String, String>(buf);
        }
        Err("Too many redirects".into())
    }).await.map_err(|e| format!("Task failed: {}", e))??;
    Ok(text)
}

#[derive(Serialize)]
struct PickedFile { name: String, text: String }

#[tauri::command]
async fn pick_pack_file(app: AppHandle) -> Result<Option<PickedFile>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("PromptKey Pack", &["json"])
        .pick_file(move |path| { let _ = tx.send(path); });
    let picked = rx.await.map_err(|e| format!("Dialog failed: {}", e))?;
    let Some(fp) = picked else { return Ok(None) };
    let path = fp.as_path().ok_or_else(|| "Invalid path".to_string())?;
    // Review F32: same size cap as direct import — read happens before preview.
    let size = std::fs::metadata(&path).map_err(|e| format!("Stat failed: {}", e))?.len();
    if size > MAX_PACK_FILE_BYTES {
        return Err(format!("Pack file too large ({} bytes > {})", size, MAX_PACK_FILE_BYTES));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Read failed: {}", e))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Some(PickedFile { name, text }))
}


#[tauri::command]
fn create_prompt(prompt: Prompt) -> Result<i32, String> {
    // 连接数据库（确保目录与表存在）
    let conn = open_db()?;
    
    // 准备插入语句
    let mut stmt = conn.prepare(
        "INSERT INTO prompts (name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    ).map_err(|e| format!("无法准备插入语句: {}", e))?;
    
    // 将tags序列化为JSON字符串
    let tags_json = prompt.tags.as_ref().map(|tags| serde_json::to_string(tags).unwrap_or_default());
    
    // 执行插入
    let id = stmt.insert(rusqlite::params![
        &prompt.name,
        &tags_json,
        &prompt.content,
        &prompt.content_type,
        &prompt.variables_json,
        &prompt.app_scopes_json,
        &prompt.inject_order,
        &prompt.version.unwrap_or(1)
    ]).map_err(|e| format!("插入失败: {}", e))?;
    
    Ok(id as i32)
}

#[tauri::command]
fn update_prompt(prompt: Prompt) -> Result<(), String> {
    // 连接数据库（确保目录与表存在）
    let conn = open_db()?;
    
    // 准备更新语句
    let mut stmt = conn.prepare(
        "UPDATE prompts SET name = ?1, tags = ?2, content = ?3, content_type = ?4, 
         variables_json = ?5, app_scopes_json = ?6, inject_order = ?7, version = ?8
         WHERE id = ?9"
    ).map_err(|e| format!("无法准备更新语句: {}", e))?;
    
    // 将tags序列化为JSON字符串
    let tags_json = prompt.tags.as_ref().map(|tags| serde_json::to_string(tags).unwrap_or_default());
    
    // 执行更新
    stmt.execute(rusqlite::params![
        &prompt.name,
        &tags_json,
        &prompt.content,
        &prompt.content_type,
        &prompt.variables_json,
        &prompt.app_scopes_json,
        &prompt.inject_order,
        &prompt.version.unwrap_or(1),
        &prompt.id
    ]).map_err(|e| format!("更新失败: {}", e))?;
    
    Ok(())
}

#[tauri::command]
fn delete_prompt(id: i32) -> Result<(), String> {
    // 连接数据库（确保目录与表存在）
    let conn = open_db()?;
    
    // 准备删除语句
    let mut stmt = conn.prepare("DELETE FROM prompts WHERE id = ?1")
        .map_err(|e| format!("无法准备删除语句: {}", e))?;
    
    // 执行删除
    stmt.execute([id])
        .map_err(|e| format!("删除失败: {}", e))?;
    
    Ok(())
}

// 打开数据库并确保目录/表存在，设置 busy_timeout 与 WAL
fn open_db() -> Result<rusqlite::Connection, String> {
    // 与 service 完全一致：从配置中读取 database_path，避免路径不一致导致“未知/0ms”
    let cfg = load_or_default_config()?;
    let database_path = cfg.database_path;
    println!("[DB] 使用数据库路径: {}", database_path);

    // 确保目录存在
    if let Some(parent) = std::path::Path::new(&database_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建数据库目录失败: {}", e))?;
    }

    let conn = rusqlite::Connection::open(&database_path)
        .map_err(|e| format!("无法连接数据库: {}", e))?;
    conn.busy_timeout(Duration::from_millis(2000))
        .map_err(|e| format!("设置 busy_timeout 失败: {}", e))?;
    // 开启 WAL（若已开启则无影响）
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| format!("设置 WAL 失败: {}", e))?;

    // 确保表存在
    conn.execute(
        "CREATE TABLE IF NOT EXISTS prompts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            tags TEXT,
            content TEXT NOT NULL,
            content_type TEXT,
            variables_json TEXT,
            app_scopes_json TEXT,
            inject_order TEXT,
            version INTEGER DEFAULT 1,
            is_pinned INTEGER DEFAULT 0,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| format!("创建 prompts 表失败: {}", e))?;

    // Ensure is_pinned column exists (for migration)
    let _ = conn.execute("ALTER TABLE prompts ADD COLUMN is_pinned INTEGER DEFAULT 0", []);
    // Phase 2 migrations: columns added later on legacy DBs
    for (col, decl) in [
        ("variables_json", "TEXT"),
        ("app_scopes_json", "TEXT"),
        ("inject_order", "TEXT"),
        ("version", "INTEGER DEFAULT 1"),
    ] {
        let _ = conn.execute(&format!("ALTER TABLE prompts ADD COLUMN {} {}", col, decl), []);
    }


    // 初始创建（可能是旧结构），后续用 ensure_usage_logs_schema 升级列
    conn.execute(
        "CREATE TABLE IF NOT EXISTS usage_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prompt_id INTEGER,
            target_app TEXT,
            window_title TEXT,
            strategy TEXT,
            success INTEGER,
            error TEXT,
            result TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| format!("创建 usage_logs 表失败: {}", e))?;

    // 确保新列存在：prompt_name、hotkey_used、injection_time_ms
    ensure_usage_logs_schema(&conn)?;

    // 创建selected_prompt表用于存储选中的提示词ID
    conn.execute(
        "CREATE TABLE IF NOT EXISTS selected_prompt (
            id INTEGER PRIMARY KEY,
            prompt_id INTEGER NOT NULL
        )",
        [],
    ).map_err(|e| format!("创建 selected_prompt 表失败: {}", e))?;
    
    // 插入默认选中记录（如果不存在）
    conn.execute(
        "INSERT OR IGNORE INTO selected_prompt (id, prompt_id) VALUES (1, 0)",
        [],
    ).map_err(|e| format!("初始化 selected_prompt 表失败: {}", e))?;

    // Phase 2: key/value app settings (default prompt policy, UI prefs mirror)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    ).map_err(|e| format!("创建 app_settings 表失败: {}", e))?;

    Ok(conn)
}

#[tauri::command]
fn set_selected_prompt(id: i32) -> Result<(), String> {
    // 连接数据库（确保目录与表存在）
    let conn = open_db()?;
    
    // 更新选中的提示词ID
    conn.execute(
        "UPDATE selected_prompt SET prompt_id = ?1 WHERE id = 1",
        rusqlite::params![id],
    ).map_err(|e| format!("设置选中提示词失败: {}", e))?;
    
    println!("设置选中提示词ID为: {}", id);
    Ok(())
}

#[tauri::command]
fn get_selected_prompt() -> Result<i32, String> {
    let conn = open_db()?;
    
    let mut stmt = conn.prepare("SELECT prompt_id FROM selected_prompt WHERE id = 1")
        .map_err(|e| format!("准备查询语句失败: {}", e))?;
    
    let mut rows = stmt.query([])
        .map_err(|e| format!("执行查询失败: {}", e))?;
    
    if let Some(row) = rows.next().map_err(|e| format!("读取查询结果失败: {}", e))? {
        let prompt_id: i32 = row.get(0).map_err(|e| format!("获取prompt_id失败: {}", e))?;
        println!("当前选中的提示词ID: {}", prompt_id);
        Ok(prompt_id)
    } else {
        println!("没有找到选中的提示词记录，返回默认值0");
        Ok(0)
    }
}

#[tauri::command]
fn get_usage_logs() -> Result<Vec<serde_json::Value>, String> {
    let conn = open_db()?;
    
    // 添加调试：检查表结构
    let mut stmt = conn.prepare("PRAGMA table_info(usage_logs)")
        .map_err(|e| format!("检查表结构失败: {}", e))?;
    let columns: Vec<String> = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(1)?) // 获取列名
    }).map_err(|e| format!("查询表结构失败: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("获取列名失败: {}", e))?;
    
    println!("数据库表结构 - 列名: {:?}", columns);
    
    let mut stmt = conn.prepare(
                        "SELECT 
            u.id,
            u.prompt_id,
            COALESCE(u.prompt_name, p.name) AS prompt_name,
            u.target_app,
            u.window_title,
            u.hotkey_used,
            u.strategy,
                        CASE 
                            WHEN u.success = 1 THEN 
                                CASE WHEN u.injection_time_ms IS NULL OR u.injection_time_ms < 1 THEN 1 ELSE u.injection_time_ms END
                            ELSE COALESCE(u.injection_time_ms, 0)
                        END AS injection_time_ms,
            u.success,
            u.error,
                u.result,
                                strftime('%s', u.created_at) AS created_at_epoch
         FROM usage_logs u
         LEFT JOIN prompts p ON p.id = u.prompt_id
         ORDER BY u.created_at DESC
         LIMIT 100"
    ).map_err(|e| format!("无法准备查询语句: {}", e))?;
    
        let log_iter = stmt.query_map([], |row| {
            // Parse epoch string to milliseconds
            let epoch_str: String = row.get(11)?;
            let epoch_secs: i64 = epoch_str.parse().unwrap_or(0);
            let created_at_ms = epoch_secs * 1000;
            
            let log_entry = serde_json::json!({
                "id": row.get::<_, i32>(0)?,
                "prompt_id": row.get::<_, Option<i32>>(1)?,
                "prompt_name": row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "未知".to_string()),
                "target_app": row.get::<_, String>(3)?,
                "window_title": row.get::<_, String>(4)?,
                "hotkey_used": row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "未知".to_string()),
                "strategy": row.get::<_, String>(6)?,
                "injection_time_ms": row.get::<_, Option<i64>>(7)?.unwrap_or(0),
                "success": row.get::<_, i32>(8)? == 1,
                "error": row.get::<_, Option<String>>(9)?,
                "result": row.get::<_, String>(10)?,
                "created_at": created_at_ms
            });        // 打印每条记录用于调试
        println!("读取到日志记录: {}", log_entry);
        
        Ok(log_entry)
    }).map_err(|e| format!("查询失败: {}", e))?;
    
    let mut logs = Vec::new();
    for log in log_iter {
        logs.push(log.map_err(|e| format!("获取日志失败: {}", e))?);
    }
    
    println!("共读取到 {} 条日志记录", logs.len());
    
    Ok(logs)
}

#[tauri::command]
fn exit_application(app: AppHandle) -> Result<(), String> {
    // 停止服务
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    if let Err(e) = service_state.stop_service() {
        eprintln!("停止服务时出错: {}", e);
    }
    
    // 退出应用
    app.exit(0);
    Ok(())
}

fn create_and_show_window(app: &AppHandle) {
    // 检查窗口是否已存在
    if let Some(existing_window) = app.get_webview_window("main") {
        let _ = existing_window.show();
        let _ = existing_window.set_focus();
        return;
    }
    
    // 创建新窗口
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("PromptKey")
        .inner_size(1000.0, 700.0)
        .min_inner_size(800.0, 600.0)
        .build()
        .unwrap();
    
    // 显示窗口
    let _ = window.show();
    let _ = window.set_focus();
}

fn toggle_window_visibility(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or_default() {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    } else {
        // 创建新窗口
        create_and_show_window(app);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    #[serde(default = "default_hotkey")]
    hotkey: String,
    #[serde(default = "default_quick_hotkey")]
    quick_hotkey: String,
    database_path: String,
    #[serde(default)]
    injection: InjectionConfig,
    // Preserve unknown/extra YAML keys (e.g. applications map) across rewrites
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InjectionConfig {
    #[serde(default = "default_injection_order")]
    order: Vec<String>,
    #[serde(default = "default_allow_clipboard")]
    allow_clipboard: bool,
    #[serde(default = "default_uia_value_pattern_mode")]
    uia_value_pattern_mode: String,
    #[serde(default = "default_true")]
    restore_clipboard: bool,
    #[serde(default = "default_true")]
    secure_gate: bool,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_yaml::Value>,
}

impl Default for InjectionConfig {
    fn default() -> Self {
        InjectionConfig {
            order: default_injection_order(),
            allow_clipboard: default_allow_clipboard(),
            uia_value_pattern_mode: default_uia_value_pattern_mode(),
            restore_clipboard: true,
            secure_gate: true,
            extra: Default::default(),
        }
    }
}

fn default_hotkey() -> String { "Ctrl+Alt+Space".into() }
fn default_quick_hotkey() -> String { "Ctrl+Alt+A".into() }
fn default_injection_order() -> Vec<String> { vec!["uia".into()] }
fn default_allow_clipboard() -> bool { true }
fn default_true() -> bool { true }
fn default_uia_value_pattern_mode() -> String { "overwrite".into() }

fn config_path() -> Result<std::path::PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| format!("读取APPDATA失败: {}", e))?;
    let dir = std::path::Path::new(&appdata).join("PromptKey");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {}", e))?;
    Ok(dir.join("config.yaml"))
}

fn load_or_default_config() -> Result<AppConfig, String> {
    let path = config_path()?;
    if path.exists() {
        let s = std::fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {}", e))?;
        let cfg: AppConfig = serde_yaml::from_str(&s).map_err(|e| format!("解析配置失败: {}", e))?;
        Ok(cfg)
    } else {
        // database_path 默认与服务一致
        let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
            format!("{}\\PromptKey\\promptmgr.db", appdata)
        } else {
            "promptmgr.db".to_string()
        };
        Ok(AppConfig {
            hotkey: default_hotkey(),
            quick_hotkey: default_quick_hotkey(),
            database_path,
            injection: InjectionConfig::default(),
            extra: Default::default(),
        })
    }
}

// Normalize a hotkey string like "ctrl+alt+f5" → "Ctrl+Alt+F5".
// The old version silently rewrote input: unknown keys fell back to a default
// main key and Ctrl+Alt were force-injected — so "F9" was saved as
// "Ctrl+Alt+Space" while the UI toasted success. Now the shared service parser
// validates strictly; anything it can't parse is a visible error.
fn canonicalize_hotkey(input: Option<String>, default: &str) -> Result<String, String> {
    let raw = input.unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(default.to_string());
    }
    let parsed = service::hotkey::parse_hotkey(trimmed)
        .map_err(|e| format!("热键无效: {}", e))?;
    if parsed.modifiers == 0 {
        // A bare key (e.g. "F9") as a global hotkey swallows normal typing —
        // refuse it explicitly rather than letting registration "work" badly.
        return Err("热键需要至少一个修饰键 (Ctrl/Alt/Shift/Win)".to_string());
    }
    Ok(parsed.canonical)
}

#[derive(Serialize)]
struct HotkeyReport {
    /// "running" | "starting" | "stopped" | "failed"
    engine: String,
    engine_error: Option<String>,
    hotkeys: Vec<service::hotkey::HotkeyCheck>,
}

/// Self-check for the global hotkeys (brief Task 3): reports what the engine
/// actually registered vs. what config asks for, plus a live availability probe
/// per combo. This is the surface that makes silent hotkey death impossible.
#[tauri::command]
fn check_hotkeys() -> Result<HotkeyReport, String> {
    let cfg = load_or_default_config()?;
    let want = [(4u32, cfg.hotkey.clone()), (5u32, cfg.quick_hotkey.clone())];

    // Registration lands within ms of engine start; when the engine just
    // (re)started, give the worker a short bounded window so the report
    // reflects the CURRENT combos, then take the snapshot as-is.
    let deadline = std::time::Instant::now() + Duration::from_millis(1200);
    loop {
        let outcomes = service::hotkey::outcomes();
        let covered = want
            .iter()
            .filter(|(_, c)| !c.is_empty())
            .all(|(id, c)| outcomes.iter().any(|(i, r)| *i == *id && r.combo == *c));
        let running = matches!(service::engine_state(), service::EngineState::Running);
        if covered || !running || std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let (engine, engine_error) = match service::engine_state() {
        service::EngineState::Running => ("running", None),
        service::EngineState::Starting => ("starting", None),
        service::EngineState::Failed(e) => ("failed", Some(e)),
        service::EngineState::Stopped => ("stopped", None),
    };
    Ok(HotkeyReport {
        engine: engine.into(),
        engine_error,
        hotkeys: service::hotkey::check(&want),
    })
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct InjectionPrefs {
    allow_clipboard: Option<bool>,
    restore_clipboard: Option<bool>,
    secure_gate: Option<bool>,
}

#[tauri::command]
fn apply_settings(
    app: AppHandle,
    hotkey: Option<String>,
    quick_hotkey: Option<String>,
    injection: Option<InjectionPrefs>,
) -> Result<String, String> {
    // 1) 读取现有配置
    let mut cfg = load_or_default_config()?;

    // 2) 规范化并写入热键 — invalid input is now a visible error, never a
    //    silent rewrite to the default.
    cfg.hotkey = canonicalize_hotkey(hotkey, "Ctrl+Alt+Space")?;
    if let Some(q) = quick_hotkey {
        cfg.quick_hotkey = canonicalize_hotkey(Some(q), "Ctrl+Alt+A")?;
    } else if cfg.quick_hotkey.is_empty() {
        cfg.quick_hotkey = default_quick_hotkey();
    }
    if let Some(inj) = injection {
        if let Some(v) = inj.allow_clipboard { cfg.injection.allow_clipboard = v; }
        if let Some(v) = inj.restore_clipboard { cfg.injection.restore_clipboard = v; }
        if let Some(v) = inj.secure_gate { cfg.injection.secure_gate = v; }
    }

    // 4) 保存 YAML
    let path = config_path()?;
    let yaml = serde_yaml::to_string(&cfg).map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, yaml).map_err(|e| format!("写入配置失败: {}", e))?;

    // 5) 平滑重启服务 — stop_service joins the old engine (which itself joins
    //    the hotkey worker after UnregisterHotKey), so the new worker never
    //    races a lingering registration. start_service errs if the engine dies.
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    let _ = service_state.stop_service();
    if let Err(e) = service_state.start_service() { return Err(e); }

    // Wait for the worker's registration verdict on the combos we just wrote;
    // a conflict/unsupported combo must reach the user, not hide in a log.
    let want = [(4u32, cfg.hotkey.clone()), (5u32, cfg.quick_hotkey.clone())];
    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    loop {
        let outs = service::hotkey::outcomes();
        let covered = want
            .iter()
            .filter(|(_, c)| !c.is_empty())
            .all(|(id, c)| outs.iter().any(|(i, r)| *i == *id && r.combo == *c));
        if covered || std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let failures: Vec<String> = service::hotkey::check(&want)
        .into_iter()
        .filter(|h| matches!(h.status.as_str(),
            "failed" | "conflict" | "unsupported" | "lost" | "not_registered"))
        .map(|h| format!(
            "{}: {}",
            h.canonical.unwrap_or_else(|| h.combo.clone()),
            h.detail.unwrap_or_else(|| h.status.clone())
        ))
        .collect();
    if !failures.is_empty() {
        return Err(format!("设置已保存，但热键注册未生效： {}", failures.join("; ")));
    }

    Ok("设置已保存并已重启服务".into())
}

#[tauri::command]
fn get_settings() -> Result<serde_json::Value, String> {
    let cfg = load_or_default_config()?;
    Ok(serde_json::json!({
        "hotkey": cfg.hotkey,
        "quick_hotkey": cfg.quick_hotkey,
        "allow_clipboard": cfg.injection.allow_clipboard,
        "restore_clipboard": cfg.injection.restore_clipboard,
        "secure_gate": cfg.injection.secure_gate,
    }))
}

#[tauri::command]
fn reset_settings(app: AppHandle) -> Result<String, String> {
    // 删除现有配置文件
    let path = config_path()?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("删除配置文件失败: {}", e))?;
    }

    // 重新创建默认配置文件
    let _ = load_or_default_config()?;

    // Restart so defaults actually take effect — previously the engine kept
    // the old hotkeys registered until a manual restart (silent staleness).
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    let _ = service_state.stop_service();
    if let Err(e) = service_state.start_service() { return Err(e); }

    Ok("设置已重置".into())
}

// 升级/补全 usage_logs 表结构，避免出现“未知/0ms”等显示问题
fn ensure_usage_logs_schema(conn: &rusqlite::Connection) -> Result<(), String> {
    // 读取当前列
    let mut stmt = conn
        .prepare("PRAGMA table_info(usage_logs)")
        .map_err(|e| format!("检查表结构失败: {}", e))?;
    let cols: Vec<String> = stmt
        .query_map([], |row| Ok(row.get::<_, String>(1)?))
        .map_err(|e| format!("查询表结构失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("获取列名失败: {}", e))?;

    let add_col = |name: &str, decl: &str| -> Result<(), String> {
        let sql = format!("ALTER TABLE usage_logs ADD COLUMN {} {}", name, decl);
        conn.execute(&sql, [])
            .map(|_| ())
            .or_else(|err| {
                // 如果列已存在或其他非致命错误，记录并忽略
                let msg = err.to_string();
                if msg.contains("duplicate column name") { Ok(()) } else { Err(format!("添加列失败 ({}): {}", name, msg)) }
            })
    };

    if !cols.iter().any(|c| c == "prompt_name") {
        add_col("prompt_name", "TEXT")?;
    }
    if !cols.iter().any(|c| c == "hotkey_used") {
        add_col("hotkey_used", "TEXT")?;
    }
    if !cols.iter().any(|c| c == "injection_time_ms") {
        add_col("injection_time_ms", "INTEGER DEFAULT 0")?;
    }
    
    // T1-001: Add columns for Quick Selection Panel
    if !cols.iter().any(|c| c == "action") {
        add_col("action", "TEXT")?;
    }
    if !cols.iter().any(|c| c == "query") {
        add_col("query", "TEXT")?;
    }

    Ok(())
}

#[tauri::command]
fn clear_usage_logs() -> Result<(), String> {
    let conn = open_db()?;
    conn.execute("DELETE FROM usage_logs", [])
        .map_err(|e| format!("清空日志失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn restart_service(app: AppHandle) -> Result<String, String> {
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    let _ = service_state.stop_service();
    std::thread::sleep(std::time::Duration::from_millis(200));
    match service_state.start_service() {
        Ok(()) => Ok("服务已重启".to_string()),
        Err(e) => Err(e)
    }
}