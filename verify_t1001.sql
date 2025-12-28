-- T1-001 Green State Verification
-- This script simulates what ensure_usage_logs_schema() does

.open %APPDATA%\\PromptKey\\promptmgr.db

-- Show current schema
.mode columns
.headers on
SELECT 'Current schema:';
PRAGMA table_info(usage_logs);

-- Check if action and query columns exist
SELECT 
    CASE WHEN COUNT(*) >= 2 THEN '✅ GREEN: Both columns exist'
         WHEN COUNT(*) = 1 THEN '⚠️ PARTIAL: One column exists'
         ELSE '❌ RED: Columns missing' 
    END AS status,
    COUNT(*) as count
FROM (
    SELECT name FROM pragma_table_info('usage_logs') WHERE name IN ('action', 'query')
);
