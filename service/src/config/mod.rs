use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    pub database_path: String,
    #[serde(default)]
    pub injection: InjectionConfig,
    #[serde(default)]
    pub applications: HashMap<String, ApplicationConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InjectionConfig {
    #[serde(default = "default_injection_order")]
    pub order: Vec<String>,
    #[serde(default = "default_allow_clipboard")]
    pub allow_clipboard: bool,
    #[serde(default = "default_uia_value_pattern_mode")]
    pub uia_value_pattern_mode: String, // "append" only
    #[serde(default = "default_debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfig {
    pub display_name: String,
    #[serde(default)]
    pub strategies: StrategyConfig,
    #[serde(default)]
    pub settings: ApplicationSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategyConfig {
    pub primary: String,
    pub fallback: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationSettings {
    #[serde(default = "default_pre_inject_delay")]
    pub pre_inject_delay: u64,
    #[serde(default = "default_focus_retry_count")]
    pub focus_retry_count: u32,
    #[serde(default = "default_verify_injection")]
    pub verify_injection: bool,
    #[serde(default = "default_use_accessibility_api")]
    pub use_accessibility_api: bool,
}

impl Default for InjectionConfig {
    fn default() -> Self {
        InjectionConfig {
            order: default_injection_order(),
            allow_clipboard: default_allow_clipboard(),
            uia_value_pattern_mode: default_uia_value_pattern_mode(), // 默认为 append
            debug_mode: default_debug_mode(),
            max_retries: default_max_retries(),
        }
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        ApplicationConfig {
            display_name: "Unknown Application".to_string(),
            strategies: StrategyConfig::default(),
            settings: ApplicationSettings::default(),
        }
    }
}

impl Default for StrategyConfig {
    fn default() -> Self {
        StrategyConfig {
            primary: "uia".to_string(),
            fallback: vec!["clipboard".to_string(), "sendinput".to_string()],
        }
    }
}

impl Default for ApplicationSettings {
    fn default() -> Self {
        ApplicationSettings {
            pre_inject_delay: default_pre_inject_delay(),
            focus_retry_count: default_focus_retry_count(),
            verify_injection: default_verify_injection(),
            use_accessibility_api: default_use_accessibility_api(),
        }
    }
}

fn default_hotkey() -> String {
    "Ctrl+Alt+Space".to_string()
}

fn default_injection_order() -> Vec<String> {
    // Updated priority: Clipboard -> SendInput (UIA removed)
    vec!["clipboard".to_string(), "sendinput".to_string()]
}

fn default_allow_clipboard() -> bool {
    true
}

fn default_uia_value_pattern_mode() -> String {
    "insert".to_string() // 默认在光标处插入
}

fn default_debug_mode() -> bool {
    false
}

fn default_max_retries() -> u32 {
    3
}

fn default_pre_inject_delay() -> u64 {
    80
}

fn default_focus_retry_count() -> u32 {
    3
}

fn default_verify_injection() -> bool {
    true
}

fn default_use_accessibility_api() -> bool {
    false
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // 获取配置文件路径
        let config_path = Self::get_config_path()?;

        // 如果配置文件不存在，则创建默认配置（不再强制删除已有配置，避免路径漂移）
        if !Path::new(&config_path).exists() {
            let default_config = Config::default_with_predefined_apps();
            default_config.save(&config_path)?;
            return Ok(default_config);
        }

        // 读取配置文件
        let content = fs::read_to_string(config_path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        // 如果应用配置为空，填充预定义配置
        if config.applications.is_empty() {
            config.applications = Self::get_predefined_applications();
        }

        // 兼容性填充：如果某些字段缺失，应用默认值（避免用户配置被覆盖）
        if config.injection.order.is_empty() {
            config.injection.order = default_injection_order();
        } else {
            // 向后兼容: 过滤掉已废弃的 "uia" 和 "textpattern_enhanced" 策略
            let original_len = config.injection.order.len();
            config.injection.order.retain(|s| {
                let sl = s.to_lowercase();
                if sl == "uia" || sl == "textpattern_enhanced" {
                    log::warn!(
                        "Ignoring deprecated strategy '{}' in config (UIA removed)",
                        s
                    );
                    false
                } else {
                    true
                }
            });
            // 如果过滤后为空，使用默认值
            if config.injection.order.is_empty() && original_len > 0 {
                log::warn!(
                    "All configured strategies were deprecated, using default: clipboard → sendinput"
                );
                config.injection.order = default_injection_order();
            }
        }
        if config.injection.uia_value_pattern_mode.is_empty() {
            config.injection.uia_value_pattern_mode = default_uia_value_pattern_mode();
        }
        // 如果 database_path 为空（历史文件），填充默认路径
        if config.database_path.trim().is_empty() {
            config.database_path = Config::default().database_path;
        }

        Ok(config)
    }

    pub fn get_config_path() -> Result<String, Box<dyn std::error::Error>> {
        // 获取APPDATA路径
        let appdata = std::env::var("APPDATA")?;
        let config_dir = format!("{}\\PromptKey", appdata);

        // 创建配置目录（如果不存在）
        fs::create_dir_all(&config_dir)?;

        Ok(format!("{}\\config.yaml", config_dir))
    }

    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(&self)?; // 添加 &self 引用
        fs::write(path, yaml)?;
        Ok(())
    }

    pub fn default_with_predefined_apps() -> Self {
        let mut config = Config::default();
        config.applications = Self::get_predefined_applications();
        config
    }

    pub fn get_predefined_applications() -> HashMap<String, ApplicationConfig> {
        let mut apps = HashMap::new();

        // VS Code配置
        apps.insert(
            "code.exe".to_string(),
            ApplicationConfig {
                display_name: "Visual Studio Code".to_string(),
                strategies: StrategyConfig {
                    primary: "textpattern_enhanced".to_string(),
                    fallback: vec!["sendinput".to_string(), "clipboard".to_string()],
                },
                settings: ApplicationSettings {
                    pre_inject_delay: 150,
                    focus_retry_count: 3,
                    verify_injection: true,
                    use_accessibility_api: false,
                },
            },
        );

        // IntelliJ IDEA配置
        apps.insert(
            "idea64.exe".to_string(),
            ApplicationConfig {
                display_name: "IntelliJ IDEA".to_string(),
                strategies: StrategyConfig {
                    primary: "clipboard".to_string(),
                    fallback: vec!["sendinput".to_string()],
                },
                settings: ApplicationSettings {
                    pre_inject_delay: 200,
                    focus_retry_count: 2,
                    verify_injection: true,
                    use_accessibility_api: true,
                },
            },
        );

        // Visual Studio配置
        apps.insert(
            "devenv.exe".to_string(),
            ApplicationConfig {
                display_name: "Visual Studio".to_string(),
                strategies: StrategyConfig {
                    primary: "uia".to_string(),
                    fallback: vec!["clipboard".to_string(), "sendinput".to_string()],
                },
                settings: ApplicationSettings {
                    pre_inject_delay: 50,
                    focus_retry_count: 2,
                    verify_injection: true,
                    use_accessibility_api: false,
                },
            },
        );

        // Notepad++配置
        apps.insert(
            "notepad++.exe".to_string(),
            ApplicationConfig {
                display_name: "Notepad++".to_string(),
                strategies: StrategyConfig {
                    primary: "textpattern_enhanced".to_string(),
                    fallback: vec!["clipboard".to_string(), "sendinput".to_string()],
                },
                settings: ApplicationSettings {
                    pre_inject_delay: 100,
                    focus_retry_count: 2,
                    verify_injection: false,
                    use_accessibility_api: false,
                },
            },
        );

        apps
    }

    pub fn get_app_config(&self, app_name: &str) -> ApplicationConfig {
        self.applications
            .get(&app_name.to_lowercase())
            .cloned()
            .unwrap_or_else(ApplicationConfig::default)
    }
}

impl Default for Config {
    fn default() -> Self {
        // 获取默认数据库路径
        let database_path = if let Ok(appdata) = std::env::var("APPDATA") {
            format!("{}\\PromptKey\\promptmgr.db", appdata)
        } else {
            "promptmgr.db".to_string() // fallback
        };

        Config {
            hotkey: default_hotkey(),
            database_path,
            injection: InjectionConfig::default(),
            applications: HashMap::new(),
        }
    }
}
