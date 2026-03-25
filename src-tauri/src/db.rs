use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Message {
    pub id: String,
    pub contact: String,
    pub chat: String,
    pub body: String,
    pub timestamp: i64,
    pub is_mine: bool,
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS messages (
            id        TEXT PRIMARY KEY,
            contact   TEXT NOT NULL,
            chat      TEXT NOT NULL,
            body      TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            is_mine   INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
    ")
}

pub fn upsert_message(conn: &Connection, msg: &Message) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO messages (id, contact, chat, body, timestamp, is_mine)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![msg.id, msg.contact, msg.chat, msg.body, msg.timestamp, msg.is_mine as i64],
    )?;
    Ok(())
}

pub fn get_recent_messages(conn: &Connection, limit: usize) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, contact, chat, body, timestamp, is_mine
         FROM messages ORDER BY timestamp DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(Message {
            id: row.get(0)?,
            contact: row.get(1)?,
            chat: row.get(2)?,
            body: row.get(3)?,
            timestamp: row.get(4)?,
            is_mine: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect()
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    match conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0)) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_setting_or(conn: &Connection, key: &str, default: &str) -> String {
    get_setting(conn, key).ok().flatten().unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_schema_created() {
        let conn = test_conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('messages','settings')",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_upsert_deduplicates() {
        let conn = test_conn();
        let msg = Message { id: "abc".into(), contact: "João".into(), chat: "João".into(), body: "Oi".into(), timestamp: 1000, is_mine: false };
        upsert_message(&conn, &msg).unwrap();
        upsert_message(&conn, &msg).unwrap(); // duplicate — should not fail or double-insert
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_recent_messages_ordered_desc() {
        let conn = test_conn();
        for (i, body) in ["first", "second", "third"].iter().enumerate() {
            upsert_message(&conn, &Message { id: i.to_string(), contact: "X".into(), chat: "X".into(), body: body.to_string(), timestamp: i as i64 * 1000, is_mine: false }).unwrap();
        }
        let msgs = get_recent_messages(&conn, 10).unwrap();
        assert_eq!(msgs[0].body, "third"); // newest first
    }

    #[test]
    fn test_settings_missing_returns_none() {
        let conn = test_conn();
        assert_eq!(get_setting(&conn, "nonexistent").unwrap(), None);
    }

    #[test]
    fn test_settings_set_and_get() {
        let conn = test_conn();
        set_setting(&conn, "sync_interval_minutes", "15").unwrap();
        assert_eq!(get_setting(&conn, "sync_interval_minutes").unwrap(), Some("15".into()));
    }

    #[test]
    fn test_settings_overwrite() {
        let conn = test_conn();
        set_setting(&conn, "key", "old").unwrap();
        set_setting(&conn, "key", "new").unwrap();
        assert_eq!(get_setting(&conn, "key").unwrap(), Some("new".into()));
    }
}
