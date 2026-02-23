use rusqlite::Connection;

use crate::core::models::{Folder, MessageSummary};

/// Initialize the database schema.
pub fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS folders (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            unread_count INTEGER DEFAULT 0,
            total_count INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS messages (
            uid INTEGER PRIMARY KEY,
            folder_path TEXT NOT NULL,
            subject TEXT,
            sender TEXT,
            date TEXT,
            is_read INTEGER DEFAULT 0,
            is_starred INTEGER DEFAULT 0,
            has_attachments INTEGER DEFAULT 0,
            thread_id INTEGER,
            body_text TEXT,
            body_html TEXT,
            FOREIGN KEY (folder_path) REFERENCES folders(path)
        );

        CREATE INDEX IF NOT EXISTS idx_messages_folder
            ON messages(folder_path, uid DESC);",
    )?;
    Ok(())
}

/// Open (or create) the cache database.
pub fn open_db() -> Result<Connection, rusqlite::Error> {
    // TODO: Use XDG data dir for persistent storage
    let conn = Connection::open_in_memory()?;
    init_db(&conn)?;
    Ok(conn)
}

/// Cache folder metadata.
pub fn save_folders(_conn: &Connection, _folders: &[Folder]) -> Result<(), rusqlite::Error> {
    // TODO: INSERT OR REPLACE into folders
    Ok(())
}

/// Load cached folders.
pub fn load_folders(_conn: &Connection) -> Result<Vec<Folder>, rusqlite::Error> {
    // TODO: SELECT * FROM folders
    Ok(Vec::new())
}

/// Cache message headers.
pub fn save_messages(
    _conn: &Connection,
    _messages: &[MessageSummary],
) -> Result<(), rusqlite::Error> {
    // TODO: Batch INSERT OR REPLACE into messages
    Ok(())
}

/// Load cached message headers for a folder.
pub fn load_messages(
    _conn: &Connection,
    _folder_path: &str,
) -> Result<Vec<MessageSummary>, rusqlite::Error> {
    // TODO: SELECT * FROM messages WHERE folder_path = ? ORDER BY uid DESC
    Ok(Vec::new())
}
