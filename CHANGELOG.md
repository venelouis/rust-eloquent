# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-05-25

### Added
- **Database-Agnostic Migration Engine:** The Artisan CLI migration runner is now entirely driver-agnostic, capable of dynamically generating standard schemas for PostgreSQL, MySQL, and SQLite identically based on the `Blueprint` builder.
- **Improved Type Safety:** Improved `.save()` internal query generation for nested fields handling generic string lengths and driver-specific Boolean types automatically.

## [1.0.0] - 2026-05-24

### Added (The Phase 3 & 4 Enterprise Expansion)
- **Constrained Eager Loading:** Added closure-constrained eager loading support (`with_posts_constrained(|q| ...)`), allowing filtering and ordering nested relations before they are mapped.
- **Global Lifecycle Observers:** Introduced a global type-safe observer pattern (`User::observe(Arc::new(UserObserverImpl))`) supporting `saving`, `saved`, `creating`, `created`, `updating`, `updated`, `deleting`, and `deleted` hooks.
- **Rust Artisan CLI:** Engineered a transaction-safe database migration and seeding CLI architecture (`run_artisan` mapping `make:migration`, `migrate`, `migrate:rollback`, and `db:seed`).
- **Subqueries & Advanced Joins:** Implemented `SubqueryBuilder` and `JoinClause` primitives allowing closure-based joins (`join_constrained`) and dynamic `EXISTS` subqueries (`where_exists`).
- **Query Logging & Debugging:** Added internal `Eloquent::enable_query_log()` and `Eloquent::disable_query_log()` to instantly intercept and print generated SQL logic to STDOUT.
- **Model Serialization & Field Hiding:** Enabled robust model JSON serialization natively compatible with `serde_json`. Added `#[eloquent(hidden)]` struct attribute to prevent sensitive columns from being exported inside `to_json()`.
- **`Json<T>` Transparency:** Extended internal wrapper `Json<T>` to natively implement `serde::Serialize` and `serde::Deserialize` for any inner struct `T`.
- **Read/Write Connection Splitting:** Added support for dedicated read replicas (`Eloquent::init_replicas`) and automatic query routing: read queries go to replicas, write operations go to the primary node.
- **Query Chunking & Cursors:** Implemented `.chunk(size, callback)` and `.chunk_with_tx(size, callback)` to process massive datasets efficiently in batches without loading everything into memory.
- **Integrated Caching Layer:** Introduced the `redis` feature flag and the `.remember(seconds)` query method to instantly cache expensive database lookups natively.
- **Background Event Hooks:** Added Redis Pub/Sub broadcasting for model lifecycle events. When models are saved or deleted, events are automatically broadcasted for external worker consumption.

### Changed
- Refactored core macro procedural code for faster compilation checks.
- Unified dependencies natively within the `rust_eloquent` framework boundary, eliminating the need for developers to pull downstream extensions like `serde` and `serde_json` manually.

## [0.1.2] - 2026-05-20
### Fixed
- Fixed module visibility scopes and standard relationships compilation.

## [0.1.1] - 2026-05-18
### Added
- Core relationships (Has Many, Belongs To, Morph Many).
- Pagination integration (`paginate(page, per_page)`).
- `sqlx` raw mappings.

## [0.1.0] - 2026-05-15
### Added
- Initial project release.
- Baseline query builder, dynamic filters, and CRUD macros.
