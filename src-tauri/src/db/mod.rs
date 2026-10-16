pub mod fts;
pub mod history;
pub mod lyrics_cache;
pub mod migrations;
pub mod playlist;
pub mod playlist_io;
pub mod queries;
pub mod session;
pub mod stream;
pub mod track_mb;

#[cfg(test)]
mod fts_tests;
#[cfg(test)]
mod playlist_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod stream_tests;
#[cfg(test)]
mod tests;

use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

/// Managed Tauri state holding the app database connection (shared with the
/// DB-backed resolvers, which need the same handle at resolve time).
pub struct DbState(pub Arc<Mutex<Connection>>);

impl DbState {
    pub fn new(conn: Connection) -> Self {
        Self(Arc::new(Mutex::new(conn)))
    }

    /// Cloneable handle for constructing resolvers at setup.
    pub fn shared(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.0)
    }
}

const SCHEMA_VERSION_TABLE: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT (datetime('now'))
)";

/// Open a database file, enable foreign keys + WAL, and run pending migrations.
pub fn open(path: &std::path::Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// In-memory connection with schema applied — used by tests.
#[cfg(test)]
pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Apply every pending migration in its own transaction.
/// Returns the versions applied by THIS call (empty on a no-op re-run).
pub fn migrate(conn: &Connection) -> Result<Vec<i64>> {
    conn.execute_batch(SCHEMA_VERSION_TABLE)?;
    let applied = fetch_applied(conn)?;
    let mut newly_applied = Vec::new();
    for migration in migrations::MIGRATIONS {
        if applied.contains(&migration.version) {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![migration.version, migration.name],
        )?;
        tx.commit()?;
        newly_applied.push(migration.version);
    }
    Ok(newly_applied)
}

fn fetch_applied(conn: &Connection) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}
