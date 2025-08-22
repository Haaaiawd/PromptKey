// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder, AppHandle
};
use std::process::Command;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

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
                Ok(Some(_)) => {
                    // 进程已退出
                    self.process = None;
                    false
                }
                Ok(None) => {
                    // 进程仍在运行
                    true
                }
                Err(_) => {
                    // 检查进程状态时出错
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
            return Ok(());
        }
        
        // 获取服务可执行文件路径
        let service_exe_path = resolve_service_exe_path()?;
        
        // 启动服务进程
        match Command::new(&service_exe_path)
            .current_dir(std::env::current_dir().unwrap())
            .spawn() {
                Ok(child) => {
                    self.process = Some(child);
                    Ok(())
                }
                Err(e) => {
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
    // 尝试从 GUI 可执行文件所在目录推导 target/{profile}/service(.exe)
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("无法获取当前可执行文件路径: {}", e))?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| "无法获取当前可执行文件目录".to_string())?;

    // 典型 dev: target/debug/prompt-manager.exe => 同目录下 service.exe
    let candidate_debug = exe_dir.join(if cfg!(windows) { "service.exe" } else { "service" });
    if candidate_debug.exists() {
        return Ok(candidate_debug.to_string_lossy().into_owned());
    }

    // 典型 release: target/release/prompt-manager.exe => 同目录下 service(.exe)
    // 若当前不是 release 目录，尝试 sibling "release"
    if let Some(target_dir) = exe_dir.parent() {
        let candidate_release_dir = target_dir.join("release");
        let candidate_release = candidate_release_dir.join(if cfg!(windows) { "service.exe" } else { "service" });
        if candidate_release.exists() {
            return Ok(candidate_release.to_string_lossy().into_owned());
        }
    }

    // 退化：尝试工作区 target/debug
    let cwd = std::env::current_dir().map_err(|e| format!("无法获取当前目录: {}", e))?;
    let fallback_debug = cwd.join("target").join("debug").join(if cfg!(windows) { "service.exe" } else { "service" });
    if fallback_debug.exists() {
        return Ok(fallback_debug.to_string_lossy().into_owned());
    }

    Err("未找到 service 可执行文件，请先构建 service 或检查路径".to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Mutex::new(ServiceState::new()))
        .invoke_handler(tauri::generate_handler![
            start_service,
            stop_service,
            check_service_status,
            apply_settings,
            get_settings,
            get_all_prompts,
            create_prompt,
            update_prompt,
            delete_prompt
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
    // 获取数据库路径
    let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
        format!("{}\\PromptManager\\promptmgr.db", appdata)
    } else {
        return Err("无法获取APPDATA路径".to_string());
    };
    
    // 连接数据库
    let conn = rusqlite::Connection::open(&database_path)
        .map_err(|e| format!("无法连接数据库: {}", e))?;
    
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
    // 获取数据库路径
    let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
        format!("{}\\PromptManager\\promptmgr.db", appdata)
    } else {
        return Err("无法获取APPDATA路径".to_string());
    };
    
    // 连接数据库
    let conn = rusqlite::Connection::open(&database_path)
        .map_err(|e| format!("无法连接数据库: {}", e))?;
    
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
    // 获取数据库路径
    let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
        format!("{}\\PromptManager\\promptmgr.db", appdata)
    } else {
        return Err("无法获取APPDATA路径".to_string());
    };
    
    // 连接数据库
    let conn = rusqlite::Connection::open(&database_path)
        .map_err(|e| format!("无法连接数据库: {}", e))?;
    
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
    // 获取数据库路径
    let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
        format!("{}\\PromptManager\\promptmgr.db", appdata)
    } else {
        return Err("无法获取APPDATA路径".to_string());
    };
    
    // 连接数据库
    let conn = rusqlite::Connection::open(&database_path)
        .map_err(|e| format!("无法连接数据库: {}", e))?;
    
    // 准备删除语句
    let mut stmt = conn.prepare("DELETE FROM prompts WHERE id = ?1")
        .map_err(|e| format!("无法准备删除语句: {}", e))?;
    
    // 执行删除
    stmt.execute([id])
        .map_err(|e| format!("删除失败: {}", e))?;
    
    Ok(())
}

fn create_and_show_window(app: &AppHandle) {
    // 创建新窗口
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Prompt Manager")
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
    let dir = std::path::Path::new(&appdata).join("PromptManager");
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
            format!("{}\\PromptManager\\promptmgr.db", appdata)
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