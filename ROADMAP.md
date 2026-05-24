# Rust Eloquent Roadmap

Our goal is to bring the best of the **Laravel Eloquent** experience to the Rust ecosystem.
Here we track the key features that differentiate Eloquent from other query builders and our implementation status.

## ✅ Implemented
- **Active Record/Models**: Structs directly connected to the database (`#[derive(Eloquent)]`).
- **Fluent Query Builder**: Method chaining (`.where_eq()`, `.order_by()`, etc).
- **Asynchronous Execution**: Powered by Tokio + SQLx.
- **Basic Magic Methods**: `.where_name("...").where_email("...")`.
- **Pagination**: `.paginate()` method to return paginated results and meta-information easily.
- **Auto Timestamps**: Native control of `created_at` and `updated_at` in `save/update/insert` methods.
- **Helper Methods**: `.first_or_fail()`, `.find_or_fail()`.
- **Pluck**: Fetching a single column.
- **Eager Loading**: N+1 problem prevention using `.with("comments")`.
- **Mutators and Accessors**: Handling data transformation via lifecycle hooks.
- **Events and Observers**: Handling hooks like `before_save`, `after_fetch`, etc.
- **Local and Global Scopes**: Reusable query constraints.
- **Soft Deletes**: Logical deletion hiding the record (`deleted_at` column).
- **Relationships**: `HasOne`, `HasMany`, `BelongsTo`, `BelongsToMany`.
- **Migrations**: Fluent schema builder API for creating tables.

## 🎉 Phase 1 Completed!
All core features of Laravel Eloquent have been successfully ported to Rust.

## 🚀 Phase 2: Advanced Features & Rust Superpowers
- [x] **Database Transactions**: Wrapping queries in transactional blocks (`Eloquent::transaction`).
- [x] **Eloquent Collections**: Custom collection struct with high-level methods (`map`, `pluck`, `key_by`).
- [x] **Compile-Time Safety**: Using Rust's strict typing and macros to check SQL columns at compile-time.
- [x] **Polymorphic Relationships**: `morphTo`, `morphMany`, `morphOne`.
- [x] **Factories and Seeders**: Fluent API for generating fake testing data.

## 👑 Phase 3: The Rust Masterpiece
- [x] **Many-to-Many Relationships**: Implement pivot table support (`belongsToMany`).
- [x] **Pagination with Metadata**: `.paginate(15)` returning total, current page, and data.
- [x] **JSON Column Casting**: `#[eloquent(json)]` macro parameter to auto-deserialize `serde_json` structs.
- [x] **Constrained Eager Loading**: Passing closures to relationships like `.with_posts_constrained(|q| q...)`.
- [x] **Rust Artisan (Migrations CLI)**: Command-line tool to generate, run, and rollback database migrations.
- [x] **Observers & Lifecycle Events**: Global observer pattern to listen to model events (`creating`, `saved`, `deleted`) externally.
- [ ] **Subqueries & Advanced Joins**: Allowing closures for complex SQL joins and subqueries.
- [ ] **Artisan Seeding (db:seed)**: Populate tables via Artisan CLI using Seeders and Factories.
- [ ] **Query Logging & Debugging**: Inspect the executed SQL directly in terminal for optimization.
- [ ] **Model Serialization (Hiding Fields)**: Attribute `#[eloquent(hidden)]` to automatically skip sensitive columns during JSON serialization.
