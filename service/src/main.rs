// Module declarations
pub mod config;
pub mod context;
pub mod db;
pub mod hotkey;
pub mod injector;
pub mod ipc;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn run_service() {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    println!("🔥 [INTERNAL_ENGINE] 提示词引擎正在子线程启动...");

    // 1. 初始化配置 (Moved up to get DB path)
    let config = crate::config::Config::load().unwrap_or_default();
    let hotkey_str = config.hotkey.clone();

    // 2. 初始化数据库
    let database = db::Database::new(&config.database_path).expect("无法初始化数据库");

    // 3. 初始化注入器
    let injector = injector::Injector::new(vec![], config.clone());

    // 3. 初始化上下文管理器
    let context_manager = context::ContextManager::new();

    // 5. 初始化热键服务
    let mut hotkey_service = hotkey::HotkeyService::new(hotkey_str);
    if let Err(e) = hotkey_service.start() {
        log::error!("无法启动热键服务: {}", e);
    }

    // 6. 初始化 IPC 客户端 (用于通知 GUI 显示窗口)
    let ipc_client = ipc::IPCClient::default();

    // 7. 初始化逻辑注入服务端 (接收来自 GUI 的直接注入请求)
    let inject_rx = crate::ipc::inject_server::start();

    // 8. 进入主循环
    println!("✅ [INTERNAL_ENGINE] 引擎就绪，等待指令...");

    // Store the context (window) that was active before opening the wheel/selector
    let mut last_active_context: Option<context::AppContext> = None;

    loop {
        // A. 检查来自 GUI 的点选注入请求
        while let Ok(prompt_id) = inject_rx.try_recv() {
            println!("🎯 [ENGINE] 收到 GUI 注入请求: ID={}", prompt_id);
            // Use the captured context if available, otherwise try to get current (fallback)
            handle_injection_request(
                &database,
                &injector,
                &context_manager,
                Some(prompt_id),
                last_active_context.as_ref(),
            );
        }

        // B. 检查热键事件
        while let Some(hotkey_id) = hotkey_service.try_wait_for_hotkey() {
            match hotkey_id {
                1 | 2 => {
                    println!("⌨️ [HOTKEY] 触发自动注入");
                    handle_injection_request(&database, &injector, &context_manager, None, None);
                }
                3 => {
                    println!("🔍 [HOTKEY] 触发搜索面板");
                    // Capture context before showing GUI
                    if let Ok(ctx) = context_manager.get_foreground_context() {
                        println!(
                            "💾 保存上下文: App={}, Title={}",
                            ctx.process_name, ctx.window_title
                        );
                        last_active_context = Some(ctx);
                    }
                    let _ = ipc_client.send_show_selector();
                }
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
                    let _ = ipc_client.send_show_wheel();
                }
                _ => {}
            }
        }

        // 防止空转
        thread::sleep(Duration::from_millis(10));
    }
}

fn handle_injection_request(
    db: &db::Database,
    injector: &injector::Injector,
    ctx: &context::ContextManager,
    force_id: Option<i32>,
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
                window_handle: windows::Win32::Foundation::HWND(std::ptr::null_mut()),
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
            println!("✨ 正在注入: [{}] {}", prompt.name, prompt.content);

            // 记录使用日志
            if let Err(e) = db.log_usage(
                prompt.id,
                &prompt.name,
                &app_name,
                &window_title,
                "Internal",
                "Internal",
                0,
                true,
                None,
                "Injected",
                action_type,
            ) {
                log::error!("无法记录使用日志: {}", e);
            }

            // 构造注入上下文
            let injection_ctx = injector::InjectionContext {
                app_name: app_name.clone(),
                window_title: window_title.clone(),
                window_handle: context.window_handle,
            };

            // 调用注入器
            if let Err(e) = injector.inject(&prompt.content, &injection_ctx) {
                log::error!("❌ 注入失败: {}", e);
                println!("❌ 注入失败: {}", e);
            } else {
                println!("✅ 注入成功");
            }
        }
        Err(e) => {
            log::error!("查询提示词失败: {}", e);
        }
    }
}

// 为了作为二进制文件运行时兼容
fn main() {
    run_service();
}
