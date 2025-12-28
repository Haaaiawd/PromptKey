use db::Prompt;
use env_logger;
use std::thread;
use std::time::Duration;

mod config;
mod context;
mod db;
mod hotkey;
mod injector;
mod ipc; // T1-007: IPC client for Service → GUI communication

fn main() {
    // 初始化日志记录器
    env_logger::init();

    log::info!("🎯 DEBUG VERSION: PromptKey service starting with DEBUG CODE...");

    // 加载配置
    let config = match config::Config::load() {
        Ok(config) => {
            log::info!("Configuration loaded successfully");
            log::debug!(
                "Config details: hotkey={}, database_path={}",
                config.hotkey,
                config.database_path
            );
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
                    log::debug!(
                        "Existing prompt: ID={}, Name={}",
                        prompt.id.unwrap_or(0),
                        prompt.name
                    );
                }
            }
        }
        Err(e) => {
            log::error!("Failed to query prompts: {}", e);
        }
    }

    // 创建注入器 (strategy is now hardcoded in injector: Clipboard -> SendInput)
    let strategies = vec![]; // Empty vec, not used anymore

    log::info!("Injection strategies: Clipboard → SendInput (hardcoded)");

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

    log::info!("PromptKey service stopped");
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
    log::info!("🚀 DEBUG: Starting injection request handler");

    // 获取配置以记录使用的热键
    let config = match config::Config::load() {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to load config for logging: {}", e);
            return;
        }
    };
    let hotkey_used = config.hotkey.clone();

    // 获取上下文信息
    let context_info = match context_manager.get_foreground_context() {
        Ok(context) => {
            log::info!(
                "Foreground context: process='{}', window='{}'",
                context.process_name,
                context.window_title
            );
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

    // 获取所有提示词并选择要使用的提示词
    match database.get_all_prompts() {
        Ok(prompts) => {
            log::info!("🔍 DEBUG: Starting prompt selection process");
            log::info!(
                "🔍 DEBUG: Found {} total prompts in database",
                prompts.len()
            );

            // 首先尝试获取选中的提示词
            let selected_prompt_id = match database.get_selected_prompt_id() {
                Ok(id) => {
                    log::info!("🔍 DEBUG: Selected prompt ID from database: {}", id);
                    id
                }
                Err(e) => {
                    log::warn!(
                        "🔍 DEBUG: Failed to get selected prompt ID: {}, using first prompt",
                        e
                    );
                    0 // 使用默认值
                }
            };

            // 根据选中的ID查找提示词
            log::info!(
                "🔍 DEBUG: Looking for prompt with selected_prompt_id={}",
                selected_prompt_id
            );
            let prompt = if selected_prompt_id > 0 {
                // 查找指定ID的提示词
                if let Some(found_prompt) =
                    prompts.iter().find(|p| p.id == Some(selected_prompt_id))
                {
                    log::info!(
                        "✅ DEBUG: Found selected prompt: {} (ID: {})",
                        found_prompt.name,
                        selected_prompt_id
                    );
                    Some(found_prompt.clone())
                } else {
                    log::warn!(
                        "❌ DEBUG: Selected prompt ID {} not found in {} prompts, using first prompt",
                        selected_prompt_id,
                        prompts.len()
                    );
                    for p in &prompts {
                        log::warn!(
                            "🔍 DEBUG: Available prompt: ID={}, Name={}",
                            p.id.unwrap_or(-1),
                            p.name
                        );
                    }
                    prompts.first().cloned()
                }
            } else {
                log::info!("🔍 DEBUG: No prompt selected (ID=0), using first prompt");
                prompts.first().cloned()
            };

            if let Some(prompt) = prompt {
                log::info!(
                    "🔥 Injecting prompt: {} using hotkey: {}",
                    prompt.name,
                    hotkey_used
                );
                log::info!("📝 Prompt content: {}", prompt.content);
                log::info!(
                    "🎯 Target: {} - {}",
                    context_info.process_name,
                    context_info.window_title
                );

                // 调试：详细打印 prompt 对象
                log::debug!(
                    "PROMPT DEBUG - ID: {:?}, Name: '{}', Content length: {}",
                    prompt.id,
                    prompt.name,
                    prompt.content.len()
                );

                // 创建注入上下文（与 injector::InjectionContext 定义匹配）
                let context = injector::InjectionContext {
                    app_name: context_info.process_name.clone(),
                    window_title: context_info.window_title.clone(),
                    window_handle: context_info.window_handle,
                };

                // 执行注入并记录详细信息
                let res = injector.inject(&prompt.content, &context);

                // 记录使用日志，包含热键信息和注入时间
                match &res {
                    Ok((strategy_used, injection_time)) => {
                        log::info!(
                            "✅ Injection successful in {}ms using hotkey: {} with strategy: {}",
                            injection_time,
                            hotkey_used,
                            strategy_used
                        );

                        // 调试：打印即将记录的数据
                        log::debug!(
                            "记录成功日志 - prompt_id: {:?}, prompt_name: '{}', strategy: '{}', time: {}ms",
                            prompt.id,
                            prompt.name,
                            strategy_used,
                            injection_time
                        );
                        // 将耗时转为至少 1ms，避免极快路径显示 0ms
                        let injection_time_ms_to_log: u128 =
                            std::cmp::max(1u64, *injection_time) as u128;
                        let log_result = database.log_usage(
                            prompt.id,
                            &prompt.name,
                            &context.app_name,
                            &context.window_title,
                            &hotkey_used,
                            strategy_used,
                            injection_time_ms_to_log,
                            true,
                            None,
                            &format!("✅ 成功注入 {}ms - 策略: {}", injection_time, strategy_used),
                        );

                        if let Err(e) = log_result {
                            log::error!("记录成功日志失败: {}", e);
                        } else {
                            log::debug!("成功记录日志到数据库");
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "❌ Injection failed using hotkey: {} - Error: {}",
                            hotkey_used,
                            e
                        );

                        // 调试：打印即将记录的错误数据
                        log::debug!(
                            "记录失败日志 - prompt_id: {:?}, prompt_name: '{}'",
                            prompt.id,
                            prompt.name
                        );

                        let log_result = database.log_usage(
                            prompt.id,
                            &prompt.name,
                            &context.app_name,
                            &context.window_title,
                            &hotkey_used,
                            "FAILED",
                            0,
                            false,
                            Some(&e.to_string()),
                            &format!("❌ 注入失败: {}", e),
                        );

                        if let Err(e) = log_result {
                            log::error!("记录失败日志失败: {}", e);
                        } else {
                            log::debug!("成功记录失败日志到数据库");
                        }
                    }
                }
            } else {
                log::warn!("❌ No prompts found in database - logging empty attempt");
                let _ = database.log_usage(
                    None,
                    "无可用提示词",
                    &context_info.process_name,
                    &context_info.window_title,
                    &hotkey_used,
                    "NO_PROMPT",
                    0,
                    false,
                    Some("No prompts available"),
                    "❌ 无可用提示词",
                );
            }
        }
        Err(e) => {
            log::error!("❌ Failed to get prompts: {} - logging error attempt", e);
            let _ = database.log_usage(
                None,
                "数据库错误",
                &context_info.process_name,
                &context_info.window_title,
                &hotkey_used,
                "DB_ERROR",
                0,
                false,
                Some(&e.to_string()),
                &format!("❌ 数据库错误: {}", e),
            );
        }
    }
}
