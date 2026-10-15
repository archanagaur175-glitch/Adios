use rusqlite::{params, Connection, Result};
use std::collections::HashMap;

/// Read all settings as a key → raw JSON string map.
pub fn settings_get_all(conn: &Connection) -> Result<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value_json FROM settings ORDER BY key")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

/// Insert or update one setting. `value_json` must be valid JSON text.
pub fn settings_set(conn: &Connection, key: &str, value_json: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
        params![key, value_json],
    )?;
    Ok(())
}
