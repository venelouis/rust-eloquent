# Rust Eloquent 🌟

![Crates.io](https://img.shields.io/crates/v/rust-eloquent?style=flat-square&color=orange)
![Docs.rs](https://img.shields.io/docsrs/rust-eloquent?style=flat-square&color=blue)
![License](https://img.shields.io/crates/l/rust-eloquent?style=flat-square&color=green)
![Databases](https://img.shields.io/badge/Databases-PostgreSQL%20%7C%20MySQL%20%7C%20SQLite-lightgrey?style=flat-square)

An Active Record ORM for Rust, inspired by Laravel's Eloquent.

Built on top of `sqlx` and procedural macros, **rust-eloquent** aims to bring the delightful and simplistic syntax of Laravel directly to the high-performance Rust ecosystem. It supports **PostgreSQL**, **MySQL**, and **SQLite** universally out of the box using dynamic driver loading!

## 🚀 Why Rust Eloquent?

In traditional Rust database handling, you have to write raw SQL queries, manage connection pools manually across every function, and bind variables repetitively. Rust Eloquent solves this by abstracting the heavy lifting behind a single `#[derive(Eloquent)]` macro.

## 🛠️ Installation

Add the library to your `Cargo.toml`:

```toml
[dependencies]
rust-eloquent = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## 📖 Quick Start

```rust
use rust_eloquent::{Eloquent, EloquentModel, sqlx::FromRow};

// 1. Just add the Eloquent macro to your struct!
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
pub struct User {
    pub id: i32, // ID = 0 means it hasn't been saved yet
    pub name: String,
    pub email: String,
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
    };
    
    user.save().await?; // Runs INSERT and updates the ID automatically!

    // 4. Update the user
    user.name = "John Doe".to_string();
    user.save().await?; // Detects ID > 0 and runs UPDATE automatically!

    // 5. Fetch from database
    let found = User::find(1).await?;
    println!("Found: {:?}", found);

    // 6. Delete
    found.delete().await?;

    Ok(())
}
```

## ✨ Available Methods

The `#[derive(Eloquent)]` macro injects an entire Query Builder into your model, allowing you to chain methods endlessly.

### 🔍 Active Record Methods
These methods are called directly on your model instance or struct:
- `Model::query()` -> Starts a new Query Builder instance.
- `Model::find(id: i32)` -> Find a single record by its Primary Key.
- `Model::all()` -> Retrieve an array containing all records.
- `model.save()` -> Automatically runs an `INSERT` or `UPDATE` depending on if the `id` is `0`.
- `model.insert()` -> Forces an `INSERT` query and updates the struct's `id`.
- `model.update()` -> Forces an `UPDATE` query based on the current `id`.
- `model.delete()` -> Deletes the record from the database.

### ⛓️ Query Builder Filters (Chainable)
You can chain these methods after calling `Model::query()` to filter your data. All values are automatically bound to prevent SQL Injection:

**AND Filters:**
- `.where_eq(column, value)`
- `.where_not_eq(column, value)`
- `.where_gt(column, value)` *(Greater than)*
- `.where_lt(column, value)` *(Less than)*
- `.where_gte(column, value)` *(Greater than or equal)*
- `.where_lte(column, value)` *(Less than or equal)*
- `.where_like(column, value)`
- `.where_not_like(column, value)`
- `.where_null(column)`
- `.where_not_null(column)`
- `.where_in(column, vec_of_values)`
- `.where_not_in(column, vec_of_values)`
- `.where_between(column, min, max)`
- `.where_not_between(column, min, max)`

**OR Filters:**
- `.or_where(column, value)`
- `.or_where_not_eq(column, value)`
- `.or_where_gt(column, value)`
- `.or_where_lt(column, value)`
- `.or_where_like(column, value)`
- `.or_where_null(column)`
- `.or_where_not_null(column)`
- `.or_where_in(column, vec_of_values)`
- `.or_where_between(column, min, max)`

### 🔢 Selecting, Grouping, Sorting & Pagination (Chainable)
- `.select(vec!["id", "name"])` -> Choose specific columns
- `.distinct()` -> Add DISTINCT clause
- `.group_by(column)` -> Add GROUP BY clause
- `.order_by(column)` -> Ascending order
- `.order_by_desc(column)` -> Descending order
- `.limit(value: usize)` -> Limit the number of results
- `.offset(value: usize)` -> Skip a number of results

### ⚡ Executors & Utilities (Terminal Methods)
End your Query Builder chain with one of these to execute the SQL query asynchronously:
- `.get().await?` -> Returns a `Vec<Model>` matching your filters.
- `.first().await?` -> Returns a single `Model` (automatically applies `LIMIT 1`). Throws `RowNotFound` if empty.
- `.count().await?` -> Returns an `i64` representing the number of rows matching your filters.
- `.delete_all().await?` -> Deletes all rows matching your filters and returns the number of rows affected.
- `.to_sql()` -> Returns a `String` containing the raw SQL query generated so far (useful for debugging).

### ✨ Dynamic Magic Methods
The macro intelligently inspects your struct fields at compile time and generates 5 exclusive methods for **each field**. 
If your struct has `email` and `name` fields, you automatically unlock:
- `.where_email(value)`
- `.or_where_email(value)`
- `.where_not_email(value)`
- `.order_by_email()`
- `.order_by_email_desc()`
- `.where_name(value)`
...and so on! This provides an incredible developer experience identical to Laravel.