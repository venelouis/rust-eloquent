#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite")))]
pub type EloquentDatabase = sqlx::Any;

#[cfg(feature = "strict-postgres")]
pub type EloquentDatabase = sqlx::Postgres;

#[cfg(feature = "strict-mysql")]
pub type EloquentDatabase = sqlx::MySql;

#[cfg(feature = "strict-sqlite")]
pub type EloquentDatabase = sqlx::Sqlite;

pub trait QueryResultExt {
    fn get_last_insert_id(&self) -> i64;
}

#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite")))]
impl QueryResultExt for sqlx::any::AnyQueryResult {
    fn get_last_insert_id(&self) -> i64 {
        self.last_insert_id().unwrap_or(0)
    }
}

#[cfg(feature = "strict-postgres")]
impl QueryResultExt for sqlx::postgres::PgQueryResult {
    fn get_last_insert_id(&self) -> i64 {
        0
    }
}

#[cfg(feature = "strict-mysql")]
impl QueryResultExt for sqlx::mysql::MySqlQueryResult {
    fn get_last_insert_id(&self) -> i64 {
        self.last_insert_id() as i64
    }
}

#[cfg(feature = "strict-sqlite")]
impl QueryResultExt for sqlx::sqlite::SqliteQueryResult {
    fn get_last_insert_id(&self) -> i64 {
        self.last_insert_rowid()
    }
}
