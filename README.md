# Rust Eloquent 🌟

![Crates.io](https://img.shields.io/crates/v/rust-eloquent?style=flat-square&color=orange)
![Docs.rs](https://img.shields.io/docsrs/rust-eloquent?style=flat-square&color=blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
![Databases](https://img.shields.io/badge/Databases-PostgreSQL%20%7C%20MySQL%20%7C%20SQLite-lightgrey?style=flat-square)
![CI](https://github.com/venelouis/rust-eloquent/actions/workflows/ci.yml/badge.svg)

An Active Record ORM for Rust, inspired by Laravel's Eloquent.

Built on top of `sqlx` and procedural macros, **rust-eloquent** aims to bring the delightful and simplistic syntax of Laravel directly to the high-performance Rust ecosystem. It supports **PostgreSQL**, **MySQL**, and **SQLite** universally out of the box using dynamic driver loading!

## 🚀 Why Rust Eloquent?

In traditional Rust database handling, you have to write raw SQL queries, manage connection pools manually across every function, and bind variables repetitively. Rust Eloquent solves this by abstracting the heavy lifting behind a single `#[derive(Eloquent)]` macro. 

**Rust Eloquent v1.1.x** brings a massive array of enterprise-grade features:
- **Read/Write Connection Splitting** for automatic scaling.
- **Integrated Redis Caching** to speed up repeating queries natively.
- **Query Chunking** for memory-safe large dataset processing.
- **Background Event Broadcasting** via Redis Pub/Sub hooks.
- **Constrained Eager Loading** for fetching deep relationships safely.
- **Global Lifecycle Observers** to intercept operations before/after they happen.
- **Subqueries & Advanced Joins** with multi-constraint `ON` clauses.
- **Artisan Migrations CLI** for auto-generating, mapping, and rolling back database schemas.
- **Dynamic STDOUT Query Logging** for rapid debugging.
- **Model Field Serialization & Hiding** to strip out secrets.

---

## 📚 Documentation & Planning

Explore our project documentation, future plans, and recent updates:
- **[Changelog](CHANGELOG.md)**: Detailed release history and updates.
- **[Roadmap v1.x](ROADMAP.md)**: Current roadmap and goals for the 1.x release cycle.
- **[Roadmap v2.0](docs/v2_roadmap.md)**: Future plans and architecture for the upcoming major release.
- **[Security & Performance Audit](docs/audit_report.md)**: Our latest internal audit and resolution notes.

---

## 🛠️ Installation

Add the library to your `Cargo.toml`:

```toml
[dependencies]
rust-eloquent = "1.1.3"
tokio = { version = "1.0", features = ["full"] }
```

## 📖 Quick Start

```rust
use rust_eloquent::{Eloquent, sqlx::FromRow};

// 1. Just add the Eloquent macro to your struct!
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")] // Optional: specifies a custom table name
pub struct User {
    pub id: i32, // ID = 0 means it hasn't been saved yet
    pub name: String,
    pub email: String,
    #[eloquent(hidden)] // This field won't be exposed when calling user.to_json()
    pub password: String,
}

#[tokio::main]
async fn main() -> Result<(), rust_eloquent::sqlx::Error> {
    // 2. Initialize the global connection pool
    Eloquent::init("sqlite::memory:").await?;

    // 3. Create a new user magically
    let mut user = User {
        id: 0,
        name: "Vene Louis".to_string(),
        email: "vene@cosmos.com".to_string(),
        password: "secret".to_string(),
    };
    
    user.save().await?; // Runs INSERT and updates the ID automatically!

    // 4. Update the user
    user.name = "John Doe".to_string();
    user.save().await?; // Detects ID > 0 and runs UPDATE automatically!

    // 5. Fetch from database
    let found = User::find(1).await?;
    println!("Found: {:?}", found);

    // 6. Delete
    found.unwrap().delete().await?;

    Ok(())
}
```

---

## ✨ Available Query Builder Methods

The `#[derive(Eloquent)]` macro injects an entire Query Builder into your model, allowing you to chain methods endlessly.

### 🔍 Active Record Methods
These methods are called directly on your model instance or struct:
- `Model::query()` -> Starts a new Query Builder instance.
- `Model::find(id: i32)` -> Find a single record by its Primary Key (returns `Option`).
- `Model::find_or_fail(id: i32)` -> Find a single record or throw `RowNotFound`.
- `Model::all()` -> Retrieve an array containing all records.
- `model.save()` -> Automatically runs an `INSERT` or `UPDATE` depending on if the `id` is `0`.
- `model.delete()` -> Deletes the record from the database.

### ⛓️ Query Filters (Chainable)
You can chain these methods after calling `Model::query()` to filter your data. All values are automatically bound to prevent SQL Injection:

**AND Filters:**
- `.where_eq(column, value)`
- `.where_not_eq(column, value)`
- `.where_gt(column, value)` / `.where_lt(column, value)`
- `.where_like(column, value)` / `.where_not_like(column, value)`
- `.where_null(column)` / `.where_not_null(column)`
- `.where_in(column, vec_of_values)`
- `.where_between(column, min, max)`

**OR Filters:**
- `.or_where(column, value)`
- `.or_where_not_eq(column, value)`
- `.or_where_like(column, value)`
- `.or_where_in(column, vec_of_values)`

### 🔢 Selection & Aggregation
- `.select_raw("users.*, posts.title")` -> Choose specific columns or aliases
- `.group_by(column)` -> Add GROUP BY clause
- `.order_by(column)` / `.order_by_desc(column)`
- `.limit(value: usize)` / `.offset(value: usize)`

### ⚡ Executors (Terminal Methods)
End your Query Builder chain with one of these to execute the SQL query asynchronously:
- `.get().await?` -> Returns a `Vec<Model>` matching your filters.
- `.first().await?` -> Returns `Option<Model>` (automatically applies `LIMIT 1`).
- `.paginate(page, per_page).await?` -> Returns `PaginationResult<Model>`.
- `.count().await?` -> Returns an `i64` representing the number of rows.
- `.delete_all().await?` -> Deletes all rows matching your filters.

---

## 🚀 Advanced Subqueries & Joins

Rust Eloquent provides powerful primitives for complex SQL joins and subqueries, maintaining `sqlx` binding safety!

### Constrained Joins
You can join tables and apply multiple exact matches inside the join clause:
```rust
let posts_with_users = Post::query()
    .join_constrained("users", |join| {
        join.on("posts.user_id", "=", "users.id")
            .on_eq("users.name", "Alice")
    })
    .where_eq("posts.status", "published")
    .get()
    .await?;
```

### Subqueries (`where_exists`)
Inject nested `WHERE EXISTS` queries natively by passing another query builder:
```rust
let active_users = User::query()
    .where_exists(
        Post::query()
            .where_column("posts.user_id", "users.id")
            .where_eq("posts.status", "published")
    )
    .get()
    .await?;
```

---

## 🛡️ Global Lifecycle Observers
You can hook into your models’ lifecycle without cluttering your structs! Create an observer and register it globally:

```rust
pub struct UserObserverImpl;

#[rust_eloquent::async_trait]
impl UserObserver for UserObserverImpl {
    async fn saving(&self, model: &mut User) -> Result<(), rust_eloquent::sqlx::Error> {
        println!("We are about to save user: {}", model.name);
        Ok(())
    }
}

// Register your observer once globally:
User::observe(Arc::new(UserObserverImpl));
```
**Supported Events**: `saving`, `saved`, `creating`, `created`, `updating`, `updated`, `deleting`, `deleted`.

---

## 🏢 Enterprise Scaling (v1.1.x)

For high-traffic applications, Rust Eloquent provides built-in enterprise features to scale your data layer.

### Read/Write Connection Splitting
Automatically route `SELECT` queries to read replicas while keeping `INSERT`/`UPDATE`/`DELETE` operations on your primary node!

```rust
// Initialize primary node
Eloquent::init("postgres://primary_db_url").await?;

// Initialize array of read replicas
Eloquent::init_replicas(&[
    "postgres://replica_1_url",
    "postgres://replica_2_url"
]).await?;

// This uses a read replica automatically (round-robin)
let users = User::all().await?;

// This uses the primary node automatically
let mut user = User::find(1).await?.unwrap();
user.name = "Updated".to_string();
user.save().await?;
```

### Redis Caching Layer
Instantly cache heavy database queries by enabling the `redis` feature flag and calling `.remember()`. 

```rust
// Initialize Redis
Eloquent::init_redis("redis://127.0.0.1/").await?;

// The first call hits the database. Subsequent calls hit Redis until the 3600 seconds expire!
let active_users = User::query()
    .where_eq("status", "active")
    .remember(3600) // Cache for 1 hour
    .get()
    .await?;
```

### Query Chunking
Process millions of records seamlessly without running out of memory using `.chunk()`.

```rust
User::query()
    .where_eq("status", "active")
    .chunk(1000, |mut batch| Box::pin(async move {
        for user in batch.iter_mut() {
            println!("Processing user: {}", user.name);
        }
        Ok(())
    }))
    .await?;
```

### Background Event Broadcasting
When you enable the `redis` feature, Rust Eloquent automatically broadcasts Pub/Sub events for model lifecycles. If you update a user, an event is emitted to Redis: `eloquent:User:updated`, carrying the updated JSON data. This is perfect for syncing external search engines or triggering background workers!

---

## 🐘 Rust Artisan CLI (Migrations & Seeding)

Ship your applications with an integrated database migration architecture running within Rust! 

```rust
// In your application's CLI entry point:
rust_eloquent::schema::run_artisan(std::env::args().collect(), vec![ /* Seeders here */ ]).await;
```

**Commands provided natively:**
- `make:migration create_users_table` -> Scaffolds a `.rs` migration file using a fluent `Blueprint` generator.
- `migrate` -> Executes un-run migrations sequentially against the database.
- `migrate:rollback` -> Undoes the previous batch of executed migrations.
- `db:seed` -> Iterates through your database Seeders.

---

## 🔎 Query Debug Logging
Ever wondered what SQL queries are running under the hood? Toggle STDOUT query logging dynamically at any point!

```rust
Eloquent::enable_query_log();
// All queries, limits, offsets, and parameter bindings will print to STDOUT
Eloquent::disable_query_log();
```

---

## ⚙️ Compile-Time Magic Methods
The macro intelligently inspects your struct fields at compile time and generates exclusive methods for **each field**. If your struct has an `email` field, you automatically unlock:
- `.where_email(value)`
- `.or_where_email(value)`
- `.where_not_email(value)`
- `.order_by_email()`
- `.order_by_email_desc()`

This provides an incredible developer experience identical to Laravel!