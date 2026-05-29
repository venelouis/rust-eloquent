use sqlx::{AnyPool, any::install_default_drivers};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

// Re-export the procedural macro so users only need to import `rust-eloquent`
pub use rust_eloquent_macros::*;
pub use sqlx;
pub use futures;
pub use serde;
pub use serde_json;

#[cfg(feature = "redis")]
pub use redis;
pub mod schema;
pub mod collection;
pub mod types;

pub use types::Json;
pub use collection::EloquentCollection;

// Re-export async_trait so the macro can use it implicitly
pub use async_trait::async_trait;

// Re-export sqlx and FromRow for database mapping
pub use sqlx::FromRow;
pub use schema::{JoinClause, SubqueryBuilder};

/// The global connection pool
static DB_POOL: OnceLock<AnyPool> = OnceLock::new();

/// The driver identifier (postgres, mysql, sqlite) to help macro syntax formatting
static DB_DRIVER: OnceLock<String> = OnceLock::new();

/// The replica connection pools for read operations
static REPLICA_POOLS: OnceLock<Vec<AnyPool>> = OnceLock::new();

/// Atomic index for replica round-robin selection
static REPLICA_INDEX: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "redis")]
static REDIS_CLIENT: OnceLock<redis::Client> = OnceLock::new();

#[cfg(feature = "redis")]
static REDIS_MANAGER: OnceLock<redis::aio::ConnectionManager> = OnceLock::new();

/// Enum dinâmico para encapsular qualquer tipo que possa ser associado ao banco de dados pelo Macro
#[derive(Clone, Debug)]
pub enum EloquentValue {
    String(String),
    Int(i32),
    Float(f64),
    Bool(bool),
}

impl From<&str> for EloquentValue {
    fn from(s: &str) -> Self { EloquentValue::String(s.to_string()) }
}
impl From<String> for EloquentValue {
    fn from(s: String) -> Self { EloquentValue::String(s) }
}
impl From<i32> for EloquentValue {
    fn from(i: i32) -> Self { EloquentValue::Int(i) }
}
impl From<f64> for EloquentValue {
    fn from(f: f64) -> Self { EloquentValue::Float(f) }
}
impl From<bool> for EloquentValue {
    fn from(b: bool) -> Self { EloquentValue::Bool(b) }
}

/// Eloquent configuration structure
pub struct Eloquent;

impl Eloquent {
    /// Initialize the global database connection pool using an agnostic URI
    pub async fn init(database_url: &str) -> Result<(), sqlx::Error> {
        install_default_drivers();
        let pool = AnyPool::connect(database_url).await?;
        
        if DB_POOL.set(pool).is_err() {
            panic!("Eloquent has already been initialized");
        }

        let driver = if database_url.starts_with("postgres") {
            "postgres"
        } else if database_url.starts_with("mysql") {
            "mysql"
        } else {
            "sqlite"
        };
        
        let _ = DB_DRIVER.set(driver.to_string());
        let _ = REPLICA_POOLS.set(vec![]);
        
        Ok(())
    }

    /// Initialize the global database connection pool and its read replicas
    pub async fn init_with_replicas(primary_url: &str, replica_urls: Vec<&str>) -> Result<(), sqlx::Error> {
        install_default_drivers();
        let pool = AnyPool::connect(primary_url).await?;
        
        if DB_POOL.set(pool).is_err() {
            panic!("Eloquent has already been initialized");
        }

        let driver = if primary_url.starts_with("postgres") {
            "postgres"
        } else if primary_url.starts_with("mysql") {
            "mysql"
        } else {
            "sqlite"
        };
        
        let _ = DB_DRIVER.set(driver.to_string());

        let mut replicas = vec![];
        for url in replica_urls {
            let p = AnyPool::connect(url).await?;
            replicas.push(p);
        }
        let _ = REPLICA_POOLS.set(replicas);
        
        Ok(())
    }

    /// Retrieve the global database connection pool (strictly for writes)
    pub fn pool() -> &'static AnyPool {
        DB_POOL.get().expect("Eloquent must be initialized before querying")
    }

    /// Retrieve the connection pool for read operations.
    /// Performs a round-robin load balancing over replicas if configured.
    pub fn read_pool() -> &'static AnyPool {
        if let Some(replicas) = REPLICA_POOLS.get()
            && !replicas.is_empty() {
                let idx = REPLICA_INDEX.fetch_add(1, Ordering::Relaxed) % replicas.len();
                return &replicas[idx];
            }
        Self::pool()
    }

    /// Retrieve the active driver string
    pub fn driver() -> &'static str {
        DB_DRIVER.get().expect("Eloquent must be initialized before querying").as_str()
    }

    /// Starts a new database transaction
    pub async fn begin_transaction() -> Result<sqlx::Transaction<'static, sqlx::Any>, sqlx::Error> {
        let pool = Self::pool();
        pool.begin().await
    }

    /// Run an array of seeders sequentially
    pub async fn seed(seeders: Vec<Box<dyn Seeder>>) -> Result<(), sqlx::Error> {
        for seeder in seeders {
            seeder.run().await?;
        }
        Ok(())
    }

    /// Enable query logging to print all queries to the terminal
    pub fn enable_query_log() {
        crate::schema::enable_query_log();
    }

    /// Disable query logging
    pub fn disable_query_log() {
        crate::schema::disable_query_log();
    }

    /// Initialize Redis connection and connection manager for caching and events
    #[cfg(feature = "redis")]
    pub async fn init_redis(redis_url: &str) -> Result<(), redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let manager = redis::aio::ConnectionManager::new(client.clone()).await?;
        let _ = REDIS_CLIENT.set(client);
        let _ = REDIS_MANAGER.set(manager);
        Ok(())
    }

    /// Get reference to the global Redis client
    #[cfg(feature = "redis")]
    pub fn redis_client() -> &'static redis::Client {
        REDIS_CLIENT.get().expect("Redis must be initialized before using cache features")
    }

    /// Get clone of the thread-safe connection manager for async Redis queries
    #[cfg(feature = "redis")]
    pub fn redis_manager() -> redis::aio::ConnectionManager {
        REDIS_MANAGER.get().expect("Redis must be initialized before using cache features").clone()
    }
}

/// A database seeder trait for populating tables
#[async_trait]
pub trait Seeder: Send + Sync {
    async fn run(&self) -> Result<(), sqlx::Error>;
}

/// The core trait that all Eloquent models will implement via #[derive(Eloquent)]
#[async_trait]
pub trait EloquentModel {
    fn table_name() -> &'static str;
}

/// Represents a paginated result set
#[derive(Debug, Clone)]
pub struct PaginationResult<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub per_page: usize,
    pub current_page: usize,
    pub last_page: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eloquent_value_conversions() {
        let v: EloquentValue = "test".into();
        assert!(matches!(v, EloquentValue::String(_)));
        let v_int: EloquentValue = 100.into();
        assert!(matches!(v_int, EloquentValue::Int(100)));
        let v_bool: EloquentValue = false.into();
        assert!(matches!(v_bool, EloquentValue::Bool(false)));
    }
}

