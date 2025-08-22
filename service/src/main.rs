use env_logger;
use std::time::Duration;
use std::thread;
use db::Prompt;

mod config;
mod db;
mod hotkey;
mod injector;
mod context;

fn main() {
    // 初始化日志记录器
    env_logger::init();
    
    log::info!("Prompt Manager service starting...");
    
    // 加载配置
    let config = match config::Config::load() {
        Ok(config) => {
            log::info!("Configuration loaded successfully");
            log::debug!("Config details: hotkey={}, database_path={}", config.hotkey, config.database_path);
            config
        }
        Err(e) => {
            log::error!("Failed to load configuration: {}", e);
            return;
        }
    };
    
    // 初始化数据库
    let database = match db::Database::new(&config.database_path) {
        Ok(database) => {
            log::info!("Database initialized successfully");
            log::debug!("Database path: {}", config.database_path);
            database
        }
        Err(e) => {
            log::error!("Failed to initialize database: {}", e);
            return;
        }
    };
    
    // 创建一个测试模板（如果数据库为空）
    match database.get_all_prompts() {
        Ok(prompts) => {
            log::info!("Found {} existing prompts in database", prompts.len());
            if prompts.is_empty() {
                let test_prompt = Prompt {
                    id: None,
                    name: "Test Prompt".to_string(),
                    tags: Some(vec!["test".to_string()]),
                    content: "This is a test prompt for MVP.".to_string(),
                    content_type: Some("text/plain".to_string()),
                    variables_json: None,
                    app_scopes_json: None,
                    inject_order: None,
                    version: Some(1),
                    updated_at: None,
                };
                
                match database.create_prompt(&test_prompt) {
                    Ok(id) => {
                        log::info!("Created test prompt with ID: {}", id);
                    }
                    Err(e) => {
                        log::error!("Failed to create test prompt: {}", e);
                    }
                }
            } else {
                for prompt in &prompts {
                    log::debug!("Existing prompt: ID={}, Name={}", prompt.id.unwrap_or(0), prompt.name);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to query prompts: {}", e);
        }
    }
    
    // 创建注入器
    let strategies = config.injection.order.iter()
        .map(|s| match s.as_str() {
            "uia" => injector::InjectionStrategy::UIA,
            "clipboard" => injector::InjectionStrategy::Clipboard,
            "sendinput" => injector::InjectionStrategy::SendInput,
            _ => {
                log::warn!("Unknown injection strategy: {}", s);
                panic!("Unknown injection strategy: {}", s);
            },
        })
        .collect::<Vec<_>>();
    
    log::info!("Injection strategies initialized: {:?}", strategies);
    
    let injector = injector::Injector::new(strategies, config.clone());
    
    // 创建上下文管理器
    let context_manager = context::ContextManager::new();
    log::debug!("Context manager created");
    
    // 创建热键服务并启动
    let mut hotkey_service = hotkey::HotkeyService::new(config.hotkey.clone());
    match hotkey_service.start() {
        Ok(_) => {
            log::info!("Hotkey service started successfully");
        }
        Err(e) => {
            log::error!("Failed to start hotkey service: {}", e);
            return;
        }
    }
    
    // 主线程监听热键事件
    log::info!("Entering main loop...");
    run_main_loop(&hotkey_service, database, injector, context_manager);
    
    // 停止热键服务
    hotkey_service.stop();
    
    log::info!("Prompt Manager service stopped");
}

fn run_main_loop(
    hotkey_service: &hotkey::HotkeyService,
    database: db::Database,
    injector: injector::Injector,
    context_manager: context::ContextManager,
) {
    log::debug!("Main loop started");
    loop {
        // 检查是否有热键事件
        let hotkey_pressed = hotkey_service.wait_for_hotkey();
        if hotkey_pressed {
            log::info!("Hotkey event detected, executing injection");
            handle_injection_request(&database, &injector, &context_manager);
        }
        
        // 短暂休眠以避免过度占用CPU
        thread::sleep(Duration::from_millis(10));
    }
}

fn handle_injection_request(
    database: &db::Database,
    injector: &injector::Injector,
    context_manager: &context::ContextManager,
) {
    log::debug!("Handling injection request");
    
    // 获取上下文信息
    let context_info = match context_manager.get_foreground_context() {
        Ok(context) => {
            log::info!("Foreground context: process='{}', window='{}'", 
                      context.process_name, context.window_title);
            context
        }
        Err(e) => {
            log::warn!("Failed to get foreground context: {}", e);
            // 使用默认上下文
            context::AppContext {
                process_name: "unknown".to_string(),
                window_title: "unknown".to_string(),
                window_handle: windows::Win32::Foundation::HWND(0 as *mut std::ffi::c_void),
            }
        }
    };
    
    // 获取第一个提示词（在MVP中我们只有一个测试提示词）
    match database.get_all_prompts() {
        Ok(prompts) => {
            if let Some(prompt) = prompts.first() {
                log::info!("Injecting prompt: {}", prompt.name);
                log::debug!("Prompt content: {}", prompt.content);
                
                // 创建注入上下文
                let context = injector::InjectionContext {
                    target_text: prompt.content.clone(),
                    app_name: context_info.process_name,
                    window_title: context_info.window_title,
                    window_handle: context_info.window_handle,
                };
                
                // 执行注入
                let start = std::time::Instant::now();
                let res = injector.inject(&prompt.content, &context);
                let dur_ms = start.elapsed().as_millis();
                match &res {
                    Ok(_) => {
                        log::info!("Injection successful");
                        let _ = database.log_usage(
                            prompt.id,
                            &context.app_name,
                            &context.window_title,
                            "UIA",
                            true,
                            None,
                            &format!("ok:{}ms", dur_ms),
                        );
                    }
                    Err(e) => {
                        log::error!("Injection failed: {}", e);
                        let _ = database.log_usage(
                            prompt.id,
                            &context.app_name,
                            &context.window_title,
                            "UIA",
                            false,
                            Some(&e.to_string()),
                            &format!("fail:{}ms", dur_ms),
                        );
                    }
                }
            } else {
                log::warn!("No prompts found in database");
            }
        }
        Err(e) => {
            log::error!("Failed to get prompts: {}", e);
        }
    }
}