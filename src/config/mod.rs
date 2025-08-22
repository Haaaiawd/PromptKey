use serde::{Deserialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub hotkey: String,
    pub database_path: String,
    pub injection: InjectionConfig,
}

#[derive(Deserialize, Debug)]
pub struct InjectionConfig {
    pub order: Vec<String>,
    pub allow_clipboard: bool,
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
    
    fn get_config_path() -> Result<String, Box<dyn std::error::Error>> {
        // 获取APPDATA路径
        let appdata = std::env::var("APPDATA")?;
        let config_dir = format!("{}\\PromptManager", appdata);
        
        // 创建配置目录（如果不存在）
        fs::create_dir_all(&config_dir)?;
        
        Ok(format!("{}\\config.yaml", config_dir))
    }
    
    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(self)?;
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
            hotkey: "Ctrl+Alt+Space".to_string(),
            database_path,
            injection: InjectionConfig {
                order: vec!["uia".to_string(), "clipboard".to_string(), "sendinput".to_string()],
                allow_clipboard: true,
            },
        }
    }
}

// 配置模块
pub fn init() {
    // 初始化配置模块
}
