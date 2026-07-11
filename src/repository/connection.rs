use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

/// Open a SQLite connection to the given path.
/// Each call creates a new connection, allowing multiple parts of the app
/// to access the database concurrently without sharing a single handle.
pub fn open_db<P: AsRef<Path>>(path: P) -> SqliteResult<Connection> {
    Connection::open(path)
}
