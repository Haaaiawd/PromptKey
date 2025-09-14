// 隐藏控制台窗口（开发/发布均不弹出）
#![windows_subsystem = "windows"]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder, AppHandle,
};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// 服务进程句柄
struct ServiceState {
    process: Option<std::process::Child>,
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

impl ServiceState {
    fn new() -> Self {
        ServiceState { process: None }
    }
    
    fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // 进程已退出
                    println!("服务进程已退出，退出状态: {:?}", status);
                    self.process = None;
                    false
                }
                Ok(None) => {
                    // 进程仍在运行
                    true
                }
                Err(e) => {
                    // 检查进程状态时出错
                    eprintln!("检查服务进程状态时出错: {}", e);
                    self.process = None;
                    false
                }
            }
        } else {
            false
        }
    }
    
    fn start_service(&mut self) -> Result<(), String> {
        if self.is_running() {
            println!("服务已在运行中");
            return Ok(());
        }
        
        // 获取服务可执行文件路径
        let service_exe_path = resolve_service_exe_path()?;
        println!("服务可执行文件路径: {}", service_exe_path);
        
        // 启动服务进程（保持日志输出，但隐藏窗口）
        let mut cmd = Command::new(&service_exe_path);
        cmd.current_dir(std::env::current_dir().unwrap())
            .stdin(Stdio::null())
            .env("RUST_LOG", "info"); // 设置日志级别
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        match cmd.spawn() {
                Ok(child) => {
                    println!("服务启动成功，PID: {:?}", child.id());
                    self.process = Some(child);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("启动服务失败: {}", e);
                    Err(format!("启动服务失败: {}", e))
                }
            }
    }
    
    fn stop_service(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.process.take() {
            // 尝试优雅地终止进程
            match child.kill() {
                Ok(_) => {
                    // 等待进程退出
                    let _ = child.wait();
                    Ok(())
                }
                Err(e) => {
                    Err(format!("停止服务失败: {}", e))
                }
            }
        } else {
            Ok(())
        }
    }
}

fn resolve_service_exe_path() -> Result<String, String> {
    // 尝试从 GUI 可执行文件所在目录推导 service(.exe) 路径
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("无法获取当前可执行文件路径: {}", e))?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| "无法获取当前可执行文件目录".to_string())?;

    let service_name = if cfg!(windows) { "service.exe" } else { "service" };
    // 1. 优先检查 Tauri 打包后的 sidecar 路径（安装后的位置）
    // 在 Tauri 打包后，sidecar 二进制文件会与主程序放在同一目录
    let packaged_service = exe_dir.join(service_name);
    if packaged_service.exists() {
        return Ok(packaged_service.to_string_lossy().into_owned());
    }

    // 2. 检查开发环境 - 同级目录下的 service.exe (debug/release)
    let candidate_same_dir = exe_dir.join(service_name);
    if candidate_same_dir.exists() {
        return Ok(candidate_same_dir.to_string_lossy().into_owned());
    }

    if let Some(target_dir) = exe_dir.parent() {
        // 如果当前在 debug，尝试 release
        let candidate_release = target_dir.join("release").join(service_name);
        if candidate_release.exists() {
            return Ok(candidate_release.to_string_lossy().into_owned());
        }
        
        // 如果当前在 release，尝试 debug  
        let candidate_debug = target_dir.join("debug").join(service_name);
        if candidate_debug.exists() {
            return Ok(candidate_debug.to_string_lossy().into_owned());
        }
    }

    // 4. 退化：尝试工作区 target/debug 和 target/release
    let cwd = std::env::current_dir().map_err(|e| format!("无法获取当前目录: {}", e))?;
    
    let fallback_debug = cwd.join("target").join("debug").join(service_name);
    if fallback_debug.exists() {
        return Ok(fallback_debug.to_string_lossy().into_owned());
    }
    
    let fallback_release = cwd.join("target").join("release").join(service_name);
    if fallback_release.exists() {
        return Ok(fallback_release.to_string_lossy().into_owned());
    }

    Err(format!(
        "未找到 service 可执行文件。已尝试的路径:\n\
         - 打包路径: {}\n\
         - 开发路径: {}\n\
         - 备用路径: {} 和 {}\n\
         请先构建 service 或检查路径配置",
        packaged_service.display(),
        candidate_same_dir.display(),
        fallback_debug.display(),
        fallback_release.display()
    ))
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
            check_service_status,
            apply_settings,
            get_settings,
            get_all_prompts,
            create_prompt,
            update_prompt,
            delete_prompt,
            reset_settings,
            set_selected_prompt,
            get_selected_prompt,
            get_usage_logs,
            exit_application,
            clear_usage_logs
        ])
        .setup(|app| {
            // 创建系统托盘菜单
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "显示/隐藏", true, None::<&str>)?;
            
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
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| format!("创建 prompts 表失败: {}", e))?;

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
            u.injection_time_ms,
            u.success,
            u.error,
            u.result,
            u.created_at
         FROM usage_logs u
         LEFT JOIN prompts p ON p.id = u.prompt_id
         ORDER BY u.created_at DESC
         LIMIT 100"
    ).map_err(|e| format!("无法准备查询语句: {}", e))?;
    
    let log_iter = stmt.query_map([], |row| {
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
            "created_at": row.get::<_, String>(11)?
        });
        
        // 打印每条记录用于调试
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
    database_path: String,
    #[serde(default)]
    injection: InjectionConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct InjectionConfig {
    #[serde(default = "default_injection_order")] 
    order: Vec<String>,
    #[serde(default = "default_allow_clipboard")] 
    allow_clipboard: bool,
    #[serde(default = "default_uia_value_pattern_mode")] 
    uia_value_pattern_mode: String,
}

fn default_hotkey() -> String { "Ctrl+Alt+Space".into() }
fn default_injection_order() -> Vec<String> { vec!["uia".into()] }
fn default_allow_clipboard() -> bool { true }
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
            database_path,
            injection: InjectionConfig::default(),
        })
    }
}

#[tauri::command]
fn apply_settings(app: AppHandle, hotkey: Option<String>, uia_mode: Option<String>) -> Result<String, String> {
    // 1) 读取现有配置
    let mut cfg = load_or_default_config()?;

    // 2) 规范化并写入热键
    if let Some(mut hk) = hotkey {
        // 简单规范化（大小写与空格）
        hk = hk.replace(" ", "");
        let lower = hk.to_lowercase();
        // 仅允许 Ctrl/Alt/Shift + 字母/数字/Space
        let allowed_main = [
            "space","a","b","c","d","e","f","g","h","i","j","k","l","m","n","o","p","q","r","s","t","u","v","w","x","y","z",
            "0","1","2","3","4","5","6","7","8","9"
        ];
        // 拆分
        let parts: Vec<&str> = lower.split('+').collect();
        let mut mods = vec![];
        let mut main: Option<&str> = None;
        for p in parts {
            match p {
                "ctrl"|"control" => mods.push("Ctrl"),
                "alt" => mods.push("Alt"),
                "shift" => mods.push("Shift"),
                other => {
                    if allowed_main.contains(&other) { main = Some(other); }
                    else { /* 非法主键，忽略 */ }
                }
            }
        }
        // 如果主键不合法，回落到 Space
        let main = main.unwrap_or("space");
        // 组装，至少包含 Ctrl+Alt
        if !mods.iter().any(|m| *m=="Ctrl") { mods.push("Ctrl"); }
        if !mods.iter().any(|m| *m=="Alt") { mods.push("Alt"); }
    let main_norm = if main == "space" { "Space".to_string() } else if main.len()==1 { main.to_uppercase() } else { main.to_string() };
    let mut parts_out = mods;
    parts_out.push(main_norm.as_str());
    cfg.hotkey = parts_out.join("+");
    } else {
        // 无输入时强制为 Ctrl+Alt+Space
        cfg.hotkey = "Ctrl+Alt+Space".into();
    }

    // 3) 写入 UIA 模式（append/overwrite），GUI 的 replace 映射为 overwrite
    if let Some(mode) = uia_mode {
        let norm = match mode.as_str() { "append"=>"append", "overwrite"|"replace"=>"overwrite", _=>"overwrite" };
        cfg.injection.uia_value_pattern_mode = norm.into();
    }

    // 4) 保存 YAML
    let path = config_path()?;
    let yaml = serde_yaml::to_string(&cfg).map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, yaml).map_err(|e| format!("写入配置失败: {}", e))?;

    // 5) 平滑重启服务
    let service_state = app.state::<Mutex<ServiceState>>();
    let mut service_state = service_state.lock().unwrap();
    let _ = service_state.stop_service();
    // 给一点时间释放热键
    std::thread::sleep(std::time::Duration::from_millis(150));
    if let Err(e) = service_state.start_service() { return Err(e); }

    Ok("设置已保存并已重启服务".into())
}

#[tauri::command]
fn get_settings() -> Result<serde_json::Value, String> {
    let cfg = load_or_default_config()?;
    Ok(serde_json::json!({
        "hotkey": cfg.hotkey,
        "uia_mode": cfg.injection.uia_value_pattern_mode,
    }))
}

#[tauri::command]
fn reset_settings() -> Result<String, String> {
    // 删除现有配置文件
    let path = config_path()?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("删除配置文件失败: {}", e))?;
    }
    
    // 重新创建默认配置文件
    let _ = load_or_default_config()?;
    
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

    Ok(())
}

#[tauri::command]
fn clear_usage_logs() -> Result<(), String> {
    let conn = open_db()?;
    conn.execute("DELETE FROM usage_logs", [])
        .map_err(|e| format!("清空日志失败: {}", e))?;
    Ok(())
}