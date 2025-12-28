// T1-001 Verification Script: Check if action/query columns exist in usage_logs

use rusqlite::{Connection, Result};

fn main() -> Result<()> {
    // Open existing DB (read-only check)
    let db_path = std::env::var("APPDATA")
        .map(|p| format!("{}\\PromptKey\\promptmgr.db", p))
        .unwrap_or_else(|_| "./promptmgr.db".to_string());

    println!("Checking DB: {}", db_path);

    let conn = Connection::open(&db_path)?;

    // Get current schema
    let mut stmt = conn.prepare("PRAGMA table_info(usage_logs)")?;
    let cols: Vec<String> = stmt
        .query_map([], |row| Ok(row.get::<_, String>(1)?))?
        .collect::<Result<Vec<_>>>()?;

    println!("\n📋 Current columns in usage_logs:");
    for col in &cols {
        println!("  - {}", col);
    }

    // RED state check: assert action and query DON'T exist
    let has_action = cols.iter().any(|c| c == "action");
    let has_query = cols.iter().any(|c| c == "query");

    println!("\n🔍 T1-001 Status:");
    println!("  - 'action' column exists: {}", has_action);
    println!("  - 'query' column exists: {}", has_query);

    if has_action && has_query {
        println!("\n✅ Task already complete (columns exist)");
        std::process::exit(0);
    } else if !has_action && !has_query {
        println!("\n❌ RED STATE CONFIRMED: Both columns missing (expected)");
        std::process::exit(1);
    } else {
        println!("\n⚠️  PARTIAL STATE: One column exists, one missing");
        std::process::exit(2);
    }
}
