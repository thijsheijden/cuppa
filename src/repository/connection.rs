use duckdb::{Connection, Result as DuckResult};

pub struct DbConnection {
    conn: Connection,
}

impl DbConnection {
    pub fn open(db_path: &str) -> DuckResult<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    pub fn execute(&self, sql: &str, params: &[&dyn duckdb::ToSql]) -> DuckResult<usize> {
        self.conn.execute(sql, params)
    }

    pub fn prepare(&self, sql: &str) -> DuckResult<duckdb::Statement<'_>> {
        self.conn.prepare(sql)
    }

    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> DuckResult<T>
    where
        P: duckdb::Params,
        F: FnOnce(&duckdb::Row<'_>) -> DuckResult<T>,
    {
        self.conn.query_row(sql, params, f)
    }
}
