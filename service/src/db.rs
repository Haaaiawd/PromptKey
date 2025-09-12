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
            std::fs::create_dir_all(parent).map_err(|e| -> Box<dyn std::error::Error> {
                Box::new(e)
            })?;
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
        
        // 创建usage_logs表
        self.conn.execute(
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
        )?;
        
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
        
        Ok(())
    }

    pub fn log_usage(
        &self,
        prompt_id: Option<i32>,
        target_app: &str,
        window_title: &str,
        strategy: &str,
        success: bool,
        error: Option<&str>,
        result: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO usage_logs (prompt_id, target_app, window_title, strategy, success, error, result)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;
        stmt.execute(rusqlite::params![
            &prompt_id,
            &target_app,
            &window_title,
            &strategy,
            &(if success { 1 } else { 0 }),
            &error,
            &result,
        ])?;
        Ok(())
    }
    
    pub fn create_prompt(&self, prompt: &Prompt) -> Result<i32, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO prompts (name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;
        
        // 将tags序列化为JSON字符串
        let tags_json = prompt.tags.as_ref().map(|tags| serde_json::to_string(tags).unwrap_or_default());
        
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
    
    #[allow(dead_code)]
    pub fn get_prompt_by_id(&self, id: i32) -> Result<Option<Prompt>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version, updated_at
             FROM prompts WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query([id])?;
        
        if let Some(row) = rows.next()? {
            // 反序列化tags字段
            let tags_str: Option<String> = row.get(2)?;
            let tags = match tags_str {
                Some(s) => {
                    match serde_json::from_str(&s) {
                        Ok(tags) => Some(tags),
                        Err(_) => None,  // 如果解析失败，忽略tags
                    }
                }
                None => None,
            };
            
            Ok(Some(Prompt {
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
            }))
        } else {
            Ok(None)
        }
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
                        Err(_) => None,  // 如果解析失败，忽略tags
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
        let mut stmt = self.conn.prepare("SELECT prompt_id FROM selected_prompt WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            let prompt_id: i32 = row.get(0)?;
            Ok(prompt_id)
        } else {
            Ok(0) // 默认返回0表示没有选中的提示词
        }
    }
    
    // 新增方法：设置选中的提示词ID
    #[allow(dead_code)]
    pub fn set_selected_prompt_id(&self, prompt_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "UPDATE selected_prompt SET prompt_id = ?1 WHERE id = 1",
            rusqlite::params![prompt_id],
        )?;
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn update_prompt(&self, prompt: &Prompt) -> Result<(), Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "UPDATE prompts SET name = ?1, tags = ?2, content = ?3, content_type = ?4, 
             variables_json = ?5, app_scopes_json = ?6, inject_order = ?7, version = ?8
             WHERE id = ?9"
        )?;
        
        // 将tags序列化为JSON字符串
        let tags_json = prompt.tags.as_ref().map(|tags| serde_json::to_string(tags).unwrap_or_default());
        
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
        ])?;
        
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn delete_prompt(&self, id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare("DELETE FROM prompts WHERE id = ?1")?;
        stmt.execute([id])?;
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }
}