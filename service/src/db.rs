use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub id: Option<i32>,
    pub name: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>, // 修改为Vec<String>以支持多标签
    pub content: String,
    pub content_type: Option<String>,
    pub variables_json: Option<String>,
    pub app_scopes_json: Option<String>,
    pub inject_order: Option<String>,
    pub version: Option<i32>,
    pub updated_at: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 确保数据库目录存在
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        }

        let conn = Connection::open(db_path)?;

        // 启用WAL模式
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let db = Database { conn };
        db.initialize_tables()?;

        Ok(db)
    }

    fn initialize_tables(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 创建prompts表
        self.conn.execute(
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
        )?;

        // 创建或迁移usage_logs表
        // 首先检查表是否存在以及是否需要迁移
        let table_info = self.conn.prepare("PRAGMA table_info(usage_logs)");
        match table_info {
            Ok(mut stmt) => {
                let rows: Result<Vec<_>, _> = stmt
                    .query_map([], |row| {
                        Ok(row.get::<_, String>(1)?) // 获取列名
                    })
                    .and_then(|iter| iter.collect());

                match rows {
                    Ok(columns) => {
                        let has_prompt_name = columns.iter().any(|col| col == "prompt_name");
                        let has_hotkey_used = columns.iter().any(|col| col == "hotkey_used");
                        let has_injection_time =
                            columns.iter().any(|col| col == "injection_time_ms");

                        if !has_prompt_name || !has_hotkey_used || !has_injection_time {
                            log::info!("检测到旧版usage_logs表结构，开始迁移...");
                            self.migrate_usage_logs_table()?;
                        } else {
                            log::debug!("usage_logs表结构已是最新版本");
                        }
                    }
                    Err(_) => {
                        // 表不存在，创建新表
                        log::info!("创建新的usage_logs表");
                        self.conn.execute(
                            "CREATE TABLE usage_logs (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                prompt_id INTEGER,
                                prompt_name TEXT,
                                target_app TEXT,
                                window_title TEXT,
                                hotkey_used TEXT,
                                strategy TEXT,
                                injection_time_ms INTEGER,
                                success INTEGER,
                                error TEXT,
                                result TEXT,
                                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                            )",
                            [],
                        )?;
                    }
                }
            }
            Err(_) => {
                // 无法检查表信息，直接创建
                self.conn.execute(
                    "CREATE TABLE IF NOT EXISTS usage_logs (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        prompt_id INTEGER,
                        prompt_name TEXT,
                        target_app TEXT,
                        window_title TEXT,
                        hotkey_used TEXT,
                        strategy TEXT,
                        injection_time_ms INTEGER,
                        success INTEGER,
                        error TEXT,
                        result TEXT,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )",
                    [],
                )?;
            }
        }

        // 创建selected_prompt表用于存储选中的提示词ID
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS selected_prompt (
                id INTEGER PRIMARY KEY,
                prompt_id INTEGER NOT NULL
            )",
            [],
        )?;

        // 插入默认选中记录（如果不存在）
        self.conn.execute(
            "INSERT OR IGNORE INTO selected_prompt (id, prompt_id) VALUES (1, 0)",
            [],
        )?;

        // TW007: Ensure action column exists in usage_logs
        let mut table_info = self.conn.prepare("PRAGMA table_info(usage_logs)")?;
        let has_action: bool = table_info
            .query_map([], |row| Ok(row.get::<_, String>(1)?))
            .ok()
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>().ok())
            .map(|cols| cols.iter().any(|col| col == "action"))
            .unwrap_or(false);

        if !has_action {
            log::info!("Adding 'action' column to usage_logs table");
            self.conn.execute(
                "ALTER TABLE usage_logs ADD COLUMN action TEXT DEFAULT 'hotkey_inject'",
                [],
            )?;
        }

        Ok(())
    }

    fn migrate_usage_logs_table(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 备份旧数据
        self.conn
            .execute("ALTER TABLE usage_logs RENAME TO usage_logs_old", [])?;

        // 创建新表结构
        self.conn.execute(
            "CREATE TABLE usage_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prompt_id INTEGER,
                prompt_name TEXT,
                target_app TEXT,
                window_title TEXT,
                hotkey_used TEXT,
                strategy TEXT,
                injection_time_ms INTEGER,
                success INTEGER,
                error TEXT,
                result TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // 迁移数据（尽可能保留旧数据）
        let migrate_sql = "
            INSERT INTO usage_logs (
                id, prompt_id, prompt_name, target_app, window_title, 
                hotkey_used, strategy, injection_time_ms, success, error, result, created_at
            )
            SELECT 
                id,
                COALESCE(prompt_id, NULL) as prompt_id,
                COALESCE(prompt_name, 'Unknown') as prompt_name,
                COALESCE(target_app, 'Unknown') as target_app,
                COALESCE(window_title, 'Unknown') as window_title,
                COALESCE(hotkey_used, 'Unknown') as hotkey_used,
                COALESCE(strategy, 'Unknown') as strategy,
                COALESCE(injection_time_ms, 0) as injection_time_ms,
                COALESCE(success, 0) as success,
                error,
                COALESCE(result, '') as result,
                COALESCE(created_at, datetime('now')) as created_at
            FROM usage_logs_old
        ";

        match self.conn.execute(migrate_sql, []) {
            Ok(count) => {
                log::info!("成功迁移 {} 条usage_logs记录", count);
                // 删除旧表
                self.conn.execute("DROP TABLE usage_logs_old", [])?;
            }
            Err(e) => {
                log::warn!("数据迁移失败: {}，将保留旧表并使用新表结构", e);
                // 如果迁移失败，保留旧表但使用新表
            }
        }

        Ok(())
    }

    pub fn log_usage(
        &self,
        prompt_id: Option<i32>,
        prompt_name: &str,
        target_app: &str,
        window_title: &str,
        hotkey_used: &str,
        strategy: &str,
        injection_time_ms: u128,
        success: bool,
        error: Option<&str>,
        result: &str,
        action: &str, // TW007: action field (e.g., 'wheel_select', 'hotkey_inject')
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 调试：打印接收到的参数
        log::debug!(
            "DB log_usage called with - prompt_id: {:?}, prompt_name: '{}', strategy: '{}', time: {}ms, action: '{}'",
            prompt_id,
            prompt_name,
            strategy,
            injection_time_ms,
            action
        );

        let mut stmt = self.conn.prepare(
            "INSERT INTO usage_logs (prompt_id, prompt_name, target_app, window_title, hotkey_used, strategy, injection_time_ms, success, error, result, action)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )?;

        let result = stmt.execute(rusqlite::params![
            &prompt_id,
            &prompt_name,
            &target_app,
            &window_title,
            &hotkey_used,
            &strategy,
            &(injection_time_ms as i64),
            &(if success { 1 } else { 0 }),
            &error,
            &result,
            &action, // TW007: Insert action value
        ]);

        match &result {
            Ok(rows_affected) => {
                log::debug!(
                    "Successfully inserted usage log, {} rows affected",
                    rows_affected
                );
            }
            Err(e) => {
                log::error!("Failed to insert usage log: {}", e);
            }
        }

        result?;
        Ok(())
    }

    pub fn create_prompt(&self, prompt: &Prompt) -> Result<i32, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO prompts (name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;

        // 将tags序列化为JSON字符串
        let tags_json = prompt
            .tags
            .as_ref()
            .map(|tags| serde_json::to_string(tags).unwrap_or_default());

        let id = stmt.insert(rusqlite::params![
            &prompt.name,
            &tags_json,
            &prompt.content,
            &prompt.content_type,
            &prompt.variables_json,
            &prompt.app_scopes_json,
            &prompt.inject_order,
            &prompt.version.unwrap_or(1)
        ])?;

        Ok(id as i32)
    }

    pub fn get_all_prompts(&self) -> Result<Vec<Prompt>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version, updated_at
             FROM prompts"
        )?;

        let rows = stmt.query_map([], |row| {
            // 反序列化tags字段
            let tags_str: Option<String> = row.get(2)?;
            let tags = match tags_str {
                Some(s) => {
                    match serde_json::from_str(&s) {
                        Ok(tags) => Some(tags),
                        Err(_) => None, // 如果解析失败，忽略tags
                    }
                }
                None => None,
            };

            Ok(Prompt {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                tags,
                content: row.get(3)?,
                content_type: row.get(4)?,
                variables_json: row.get(5)?,
                app_scopes_json: row.get(6)?,
                inject_order: row.get(7)?,
                version: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut prompts = Vec::new();
        for prompt in rows {
            prompts.push(prompt?);
        }

        Ok(prompts)
    }

    // 新增方法：获取选中的提示词ID
    pub fn get_selected_prompt_id(&self) -> Result<i32, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT prompt_id FROM selected_prompt WHERE id = 1")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let prompt_id: i32 = row.get(0)?;
            Ok(prompt_id)
        } else {
            Ok(0) // 默认返回0表示没有选中的提示词
        }
    }

    pub fn get_prompt_by_id(&self, id: i32) -> Result<Prompt, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version, updated_at
             FROM prompts WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([id], |row| {
            let tags_str: Option<String> = row.get(2)?;
            let tags = match tags_str {
                Some(s) => serde_json::from_str(&s).ok(),
                None => None,
            };

            Ok(Prompt {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                tags,
                content: row.get(3)?,
                content_type: row.get(4)?,
                variables_json: row.get(5)?,
                app_scopes_json: row.get(6)?,
                inject_order: row.get(7)?,
                version: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        if let Some(prompt) = rows.next() {
            Ok(prompt?)
        } else {
            Err("Prompt not found".into())
        }
    }

    pub fn find_prompt_for_context(
        &self,
        _app_name: &str,
        _window_title: &str,
    ) -> Result<Option<Prompt>, Box<dyn std::error::Error>> {
        let selected_id = self.get_selected_prompt_id()?;
        if selected_id == 0 {
            return Ok(None);
        }
        match self.get_prompt_by_id(selected_id) {
            Ok(p) => Ok(Some(p)),
            Err(_) => Ok(None),
        }
    }
}
