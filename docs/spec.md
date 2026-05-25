# rust-eloquent Specification 📄
### *"The Single Source of Truth (SST) for ORM Architecture & Macros"*

This document is the **Single Source of Truth (SST)** for the **rust-eloquent ORM**. It specifies the exact macros, attributes, query builders, and database connection architectures available in `rust-eloquent`.

> [!IMPORTANT]
> **AI Alignment Instruction:**
> Whenever updating, refactoring, or generating code for applications using rust-eloquent, **always** refer to this specification as the baseline. Do not invent or assume macro parameters or query patterns outside of this document.

---

## 📂 1. Model Definition & Macro Attributes

All Active Record entities are defined as Rust structs deriving `rust_eloquent::Eloquent` and mapping to sqlx rows:

```rust
use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};

#[derive(Debug, Clone, FromRow, Eloquent)]
#[eloquent(
    table = "table_name",           // Map to custom database table (defaults to lowercase plural struct name)
    global_scope = "my_scope",     // Apply query filter scope globally to all SELECTs
    before_save = "saving_hook",   // Method called before saving (creating or updating)
    after_save = "saved_hook",     // Method called after saving
    before_delete = "deleting_hook",// Method called before deletion
    after_delete = "deleted_hook", // Method called after deletion
    after_fetch = "loaded_hook"    // Method called after fetching records
)]
pub struct BlogPost {
    pub id: i32,
    pub title: String,
    
    #[eloquent(hidden)]            // Skip this field during JSON serialization
    pub secret_token: String,
    
    pub created_at: String,        // Automatic auto-timestamp
    pub updated_at: String,        // Automatic auto-timestamp
    pub deleted_at: Option<String>,// Triggers soft-delete mode when present in struct
}
```

---

## 🔗 2. Declarative Relationships

Declare relationships directly on model fields using custom `#[eloquent(...)]` attributes. The derive macro generates both direct fetching futures and eager loading hooks.

### 2.1. One-to-Many (`has_many`)
One record owns multiple child records.
```rust
#[eloquent(has_many = "Comment", foreign_key = "post_id", local_key = "id")]
pub comments: Option<Vec<Comment>>,
```

### 2.2. One-to-One (`has_one`)
One record owns exactly one child record.
```rust
#[eloquent(has_one = "Profile", foreign_key = "user_id", local_key = "id")]
pub profile: Option<Profile>,
```

### 2.3. Inverse Relationship (`belongs_to`)
A child record belongs to a parent record.
```rust
#[eloquent(belongs_to = "User", foreign_key = "user_id", related_key = "id")]
pub user: Option<User>,
```

### 2.4. Many-to-Many (`belongs_to_many`)
Records linked through an intermediate pivot table.
```rust
#[eloquent(belongs_to_many = "Role", pivot_table = "role_user", foreign_key = "user_id", related_key = "role_id", local_key = "id")]
pub roles: Option<Vec<Role>>,
```

### 2.5. Polymorphic One-to-Many (`morph_many`)
A target model belongs to more than one type of model on a single association.
```rust
#[eloquent(morph_many = "Comment", name = "commentable", local_key = "id")]
pub comments: Option<Vec<Comment>>,
```
*Creates column checks for `<name>_type` and `<name>_id` on the target table (e.g. `commentable_type = "BlogPost"` and `commentable_id = blog_post.id`).*

### 2.6. Polymorphic One-to-One (`morph_one`)
A target model belongs to more than one type of model on a single association.
```rust
#[eloquent(morph_one = "Image", name = "imageable", local_key = "id")]
pub image: Option<Image>,
```

---

## ⚡ 3. Fluent Query Builder API

`Model::query()` returns a compiled `ModelQueryBuilder` that supports chainable queries, subqueries, and replica routing.

### 3.1. Conditional Comparisons
* `.where_eq(column, value)`
* `.where_not_eq(column, value)`
* `.where_gt(column, value)`
* `.where_lt(column, value)`
* `.where_gte(column, value)`
* `.where_lte(column, value)`
* `.where_like(column, value)`
* `.where_null(column)`
* `.where_not_null(column)`
* `.where_in(column, Vec<values>)`
* `.where_not_in(column, Vec<values>)`
* `.where_between(column, min, max)`
* `.where_not_between(column, min, max)`
* `.or_where(column, value)`

### 3.2. Scopes, Sorting & Limits
* `.take(limit: usize)` / `.limit(limit: usize)`
* `.skip(offset: usize)` / `.offset(offset: usize)`
* `.latest(column)` / `.oldest(column)`
* `.order_by(column)` / `.order_by_desc(column)`

### 3.3. Joins & Aggregates
* `.join(table, first, operator, second)`
* `.left_join(table, first, operator, second)`
* `.join_constrained(table, |join_clause| ...)`
* `.where_exists(subquery)`

### 3.4. Cache Integration
* `.remember(seconds: u32)`: Automatically cache query results in Redis for the specified TTL.

### 3.5. Eager Loading (N+1 Prevention)
* `.with_comments()`: Load comments relation.
* `.with_comments_constrained(|q| q.where_eq("approved", true))`: Load relation applying filter.

---

## 📈 4. Pagination & Results

* `.get().await` -> `Result<Vec<Model>, sqlx::Error>`
* `.first().await` -> `Result<Option<Model>, sqlx::Error>`
* `.find(id).await` -> `Result<Model, sqlx::Error>`
* `.paginate(page: usize, per_page: usize).await` -> `Result<PaginationResult<Model>, sqlx::Error>`
  ```rust
  pub struct PaginationResult<T> {
      pub data: Vec<T>,
      pub total: i64,
      pub per_page: usize,
      pub current_page: usize,
      pub last_page: usize,
  }
  ```

---

## 🏭 5. Connection Pools & Replication splitting

Agnostic database connection management split cleanly between writes (primary node) and reads (load balanced round-robin replicas).

* **Single connection:**
  ```rust
  Eloquent::init("sqlite://rullst.db").await?;
  ```
* **Primary / Replica routing:**
  ```rust
  Eloquent::init_with_replicas(
      "postgres://primary-db.host/prod",
      vec!["postgres://replica-1.host/prod", "postgres://replica-2.host/prod"]
  ).await?;
  ```
* **Query splitting:**
  * All builder execution methods like `.get()`, `.first()`, `.paginate()` dynamically fetch from `Eloquent::read_pool()`.
  * All mutative operations like `.save()`, `.delete()`, `.begin_transaction()` dynamically fetch from `Eloquent::pool()`.

---

## 🧪 6. Factories, Observers & Seeders

### 6.1. Entity Factories
Fluent generation of fake testing data:
```rust
let users = User::factory(|| User {
    id: 0,
    name: "Fake User".to_string(),
})
.count(5)
.create()
.await?;
```

### 6.2. Observers
Attach lifecycle listeners externally:
```rust
#[rust_eloquent::async_trait]
pub trait UserObserver: Send + Sync {
    async fn creating(&self, model: &mut User) -> Result<(), sqlx::Error>;
    async fn created(&self, model: &User) -> Result<(), sqlx::Error>;
}
```

### 6.3. Seeders
Standard populate traits:
```rust
#[rust_eloquent::async_trait]
impl Seeder for DatabaseSeeder {
    async fn run(&self) -> Result<(), sqlx::Error> {
        User::factory(|| User { ... }).count(10).create().await?;
        Ok(())
    }
}
```
