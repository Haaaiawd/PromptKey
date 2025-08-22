use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    pub database_path: String,
    #[serde(default)]
    pub injection: InjectionConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InjectionConfig {
    #[serde(default = "default_injection_order")]
    pub order: Vec<String>,
    #[serde(default = "default_allow_clipboard")]
    pub allow_clipboard: bool,
    #[serde(default = "default_uia_value_pattern_mode")]
    pub uia_value_pattern_mode: String, // "append" only
}

impl Default for InjectionConfig {
    fn default() -> Self {
        InjectionConfig {
            order: default_injection_order(),
            allow_clipboard: default_allow_clipboard(),
            uia_value_pattern_mode: default_uia_value_pattern_mode(), // 默认为 append
        }
    }
}

fn default_hotkey() -> String {
    "Ctrl+Alt+Space".to_string()
}

fn default_injection_order() -> Vec<String> {
    // 按当前偏好固定为仅 UIA
    vec!["uia".to_string()]
}

fn default_allow_clipboard() -> bool {
    true
}

fn default_uia_value_pattern_mode() -> String {
    "append".to_string() // 只允许 append 模式
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // 获取配置文件路径
    let config_path = Self::get_config_path()?;
        
        // 如果配置文件不存在，则创建默认配置
        if !Path::new(&config_path).exists() {
            let default_config = Config::default();
            default_config.save(&config_path)?;
            return Ok(default_config);
        }
        
        // 读取配置文件
        let content = fs::read_to_string(config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn get_config_path() -> Result<String, Box<dyn std::error::Error>> {
        // 获取APPDATA路径
        let appdata = std::env::var("APPDATA")?;
        let config_dir = format!("{}\\PromptManager", appdata);
        
        // 创建配置目录（如果不存在）
        fs::create_dir_all(&config_dir)?;
        
        Ok(format!("{}\\config.yaml", config_dir))
    }
    
    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(&self)?;  // 添加 &self 引用
        fs::write(path, yaml)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        // 获取默认数据库路径
        let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
            format!("{}\\PromptManager\\promptmgr.db", appdata)
        } else {
            "promptmgr.db".to_string() // fallback
        };
        
        Config {
            hotkey: default_hotkey(),
            database_path,
            injection: InjectionConfig::default(),
        }
    }
}