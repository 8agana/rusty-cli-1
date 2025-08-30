use crate::api::Message;
use anyhow::Result;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub struct SessionStore;

impl SessionStore {
    fn data_dir() -> PathBuf {
        let mut dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        dir.push("rusty-cli");
        dir
    }
    fn db_path() -> PathBuf {
        Self::data_dir().join("sessions.db")
    }
    fn now() -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "".into())
    }

    fn conn() -> Result<Connection> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;\n
             CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);\n
             CREATE TABLE IF NOT EXISTS messages (
               session_id TEXT NOT NULL,
               idx INTEGER NOT NULL,
               role TEXT NOT NULL,
               content TEXT,
               name TEXT,
               tool_call_id TEXT,
               PRIMARY KEY(session_id, idx),
               FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
             );\n
             CREATE TABLE IF NOT EXISTS undelete (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               original_path TEXT NOT NULL,
               backup_path TEXT NOT NULL,
               deleted_at TEXT NOT NULL
             );\n
             CREATE TABLE IF NOT EXISTS notes (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               title TEXT,
               content TEXT NOT NULL,
               tags TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );",
        )?;
        Ok(conn)
    }

    pub fn conn_ro() -> Result<Connection> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(conn)
    }

    pub fn conn_rw() -> Result<Connection> {
        Self::conn()
    }

    pub fn last() -> Result<Option<String>> {
        let conn = Self::conn()?;
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(id)
    }

    pub fn load(id: &str) -> Result<Vec<Message>> {
        let conn = Self::conn()?;
        let mut stmt = conn.prepare(
            "SELECT role, content, name, tool_call_id FROM messages WHERE session_id=? ORDER BY idx ASC",
        )?;
        let rows = stmt.query_map([id], |r| {
            Ok(Message {
                role: r.get(0)?,
                content: r.get::<_, Option<String>>(1)?,
                tool_calls: None,
                tool_call_id: r.get(3)?,
            })
        })?;
        let mut out = vec![];
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn save(id: &str, messages: &[Message]) -> Result<()> {
        let mut conn = Self::conn()?;
        let now = Self::now();
        conn.execute(
            "INSERT OR IGNORE INTO sessions (id, created_at, updated_at) VALUES (?, ?, ?)",
            params![id, now, now],
        )?;
        conn.execute(
            "UPDATE sessions SET updated_at=? WHERE id=?",
            params![now, id],
        )?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM messages WHERE session_id=?", params![id])?;
        for (i, m) in messages.iter().enumerate() {
            tx.execute(
                "INSERT INTO messages (session_id, idx, role, content, name, tool_call_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, i as i64, m.role, m.content, None::<String>, m.tool_call_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn record_deleted(original_path: &str, backup_path: &str) -> Result<()> {
        let conn = Self::conn()?;
        let now = Self::now();
        conn.execute(
            "INSERT INTO undelete (original_path, backup_path, deleted_at) VALUES (?, ?, ?)",
            params![original_path, backup_path, now],
        )?;
        Ok(())
    }

    pub fn pop_latest_deleted(original_path: &str) -> Result<Option<String>> {
        let conn = Self::conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, backup_path FROM undelete WHERE original_path = ? ORDER BY id DESC LIMIT 1",
        )?;
        let row: Option<(i64, String)> = stmt
            .query_row([original_path], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        if let Some((id, backup)) = row {
            conn.execute("DELETE FROM undelete WHERE id = ?", params![id])?;
            Ok(Some(backup))
        } else {
            Ok(None)
        }
    }

    pub fn backups_dir() -> PathBuf {
        Self::data_dir().join("undelete")
    }

    pub fn list_deleted(limit: usize) -> Result<Vec<(String, String)>> {
        let conn = Self::conn()?;
        let mut stmt = conn
            .prepare("SELECT original_path, deleted_at FROM undelete ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit as i64], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let mut out = vec![];
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}
