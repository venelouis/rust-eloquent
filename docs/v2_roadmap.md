# Rust Eloquent - Version 2.0.0 Roadmap

This document outlines the planned breaking changes and architectural upgrades for the next major release (`v2.0.0`). These changes were intentionally deferred from the `v1.x` branch to preserve backward compatibility and maintain the simplistic, lifetime-free API.

## 1. ⚡ Zero-Copy Query Builder (String Optimization)

**Current State (v1.x):**
The `QueryBuilder` allocates new `String` objects on the heap using `format!` for every condition (e.g., `where_eq`, `join`, `order_by`). This was done to keep the API simple and avoid polluting the builder and `ActiveRecord` implementation with generic lifetimes (`<'a>`).

**Proposed Change (v2.0):**
Refactor the internal `wheres`, `joins`, and `selects` collections to use `std::borrow::Cow<'a, str>`.
- This will completely eliminate heap allocations for static column names and SQL fragments.
- **Breaking Change:** The `QueryBuilder` struct will require a lifetime parameter `QueryBuilder<'a>`. All functions returning or chaining the builder will need to declare this lifetime, cascading into the asynchronous `Future` bounds of `ActiveRecord` methods.
- **Implementation Strategy:** This profound transition will be implemented iteratively on the `dev` branch to ensure we can solve the complex lifetime cascades before enforcing it on end users.

## 2. 🛡️ Strict SQL Typing (Via Feature Flags)

**Current State (v1.x):**
The library uses `sqlx::AnyPool` and a custom generic enum (`EloquentValue`) to map types dynamically at runtime. This allows the ORM to connect to PostgreSQL, MySQL, and SQLite seamlessly without changing the Rust codebase. However, it sacrifices Rust's powerful compile-time SQL verification.

**Proposed Change (v2.0):**
Introduce an optional "Strict Mode" via Cargo **Feature Flags** (e.g., `features = ["strict-postgres"]`).
- **Strategic Update:** Instead of removing `AnyPool` entirely and breaking compatibility for all current users, the `v1.x` dynamic mode will remain available.
- When the strict feature flag is enabled, the ORM will inject strongly-typed executors (`PgPool`, `MySqlPool`, `SqlitePool`) directly into the AST generation. All internal query builders and connection handlers will drop `sqlx::Any` and statically map parameters natively to the compiled target driver, eliminating runtime conversion errors and performance overhead.
- This dual-approach provides a safe migration path for existing applications while offering maximum safety for new projects.

## 3. 🧹 Automated Resource Cleanup (Subquery Scopes)

**Current State (v1.x):**
Subqueries and raw scope injections do not automatically drop their memory footprints until the parent query completes execution.

**Proposed Change (v2.0):**
Implement custom `Drop` traits or an explicit arena allocator for complex query chains to reduce the maximum memory footprint during large `EXISTS` subquery resolutions.
