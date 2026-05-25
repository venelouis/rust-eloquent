use sqlx::Error;

pub struct Column {
    pub name: String,
    pub col_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub default_value: Option<String>,
}

impl Column {
    pub fn new(name: &str, col_type: &str) -> Self {
        Self {
            name: name.to_string(),
            col_type: col_type.to_string(),
            is_nullable: true,
            is_primary_key: false,
            is_auto_increment: false,
            default_value: None,
        }
    }

    pub fn not_null(&mut self) -> &mut Self {
        self.is_nullable = false;
        self
    }

    pub fn nullable(&mut self) -> &mut Self {
        self.is_nullable = true;
        self
    }

    pub fn default(&mut self, val: &str) -> &mut Self {
        self.default_value = Some(val.to_string());
        self
    }

    pub fn primary(&mut self) -> &mut Self {
        self.is_primary_key = true;
        self
    }
}

pub struct Blueprint {
    pub columns: Vec<Column>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self { columns: vec![] }
    }

    pub fn id(&mut self) -> &mut Column {
        self.columns.push(Column {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            is_nullable: false,
            is_primary_key: true,
            is_auto_increment: true,
            default_value: None,
        });
        self.columns.last_mut().unwrap()
    }

    pub fn string(&mut self, name: &str) -> &mut Column {
        let col = Column::new(name, "TEXT");
        self.columns.push(col);
        self.columns.last_mut().unwrap()
    }

    pub fn integer(&mut self, name: &str) -> &mut Column {
        let col = Column::new(name, "INTEGER");
        self.columns.push(col);
        self.columns.last_mut().unwrap()
    }

    pub fn float(&mut self, name: &str) -> &mut Column {
        let col = Column::new(name, "REAL");
        self.columns.push(col);
        self.columns.last_mut().unwrap()
    }

    pub fn boolean(&mut self, name: &str) -> &mut Column {
        let col = Column::new(name, "INTEGER");
        self.columns.push(col);
        self.columns.last_mut().unwrap()
    }

    pub fn timestamps(&mut self) {
        let mut created = Column::new("created_at", "TEXT");
        created.default("CURRENT_TIMESTAMP");
        self.columns.push(created);
        
        let mut updated = Column::new("updated_at", "TEXT");
        updated.default("CURRENT_TIMESTAMP");
        self.columns.push(updated);
    }

    pub fn soft_deletes(&mut self) {
        let col = Column::new("deleted_at", "TEXT");
        self.columns.push(col);
        self.columns.last_mut().unwrap().nullable();
    }
    
    pub fn build(&self) -> String {
        let mut defs = vec![];
        for col in &self.columns {
            let mut def = format!("{} {}", col.name, col.col_type);
            if col.is_primary_key {
                def.push_str(" PRIMARY KEY");
            }
            if col.is_auto_increment {
                def.push_str(" AUTOINCREMENT");
            }
            if !col.is_nullable && !col.is_primary_key {
                def.push_str(" NOT NULL");
            }
            if let Some(val) = &col.default_value {
                def.push_str(&format!(" DEFAULT {}", val));
            }
            defs.push(def);
        }
        defs.join(",\n    ")
    }
}

pub struct Schema;

impl Schema {
    pub async fn create<F>(table_name: &str, callback: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Blueprint),
    {
        let mut blueprint = Blueprint::new();
        callback(&mut blueprint);
        
        let columns_sql = blueprint.build();
        let sql = format!("CREATE TABLE IF NOT EXISTS {} (\n    {}\n);", table_name, columns_sql);
        
        let pool = crate::Eloquent::pool();
        sqlx::query(&sql).execute(pool).await?;
        
        Ok(())
    }
    
    pub async fn drop_if_exists(table_name: &str) -> Result<(), Error> {
        let sql = format!("DROP TABLE IF EXISTS {};", table_name);
        let pool = crate::Eloquent::pool();
        sqlx::query(&sql).execute(pool).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait Migration: Send + Sync {
    fn name(&self) -> &'static str;
    async fn up(&self) -> Result<(), Error>;
    async fn down(&self) -> Result<(), Error>;
}

pub async fn run_artisan_with_args(
    args: &[String],
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn crate::Seeder>>
) -> Result<(), Error> {
    if args.len() < 2 {
        println!("Rust Eloquent Artisan CLI");
        println!("Usage:");
        println!("  make:migration <name>   Generate a new migration");
        println!("  migrate                  Run all pending migrations");
        println!("  migrate:rollback         Rollback the last batch of migrations");
        println!("  status                   Show migrations status");
        println!("  db:seed                  Populate the database with seeders");
        return Ok(());
    }

    let command = &args[1];
    match command.as_str() {
        "make:migration" => {
            if args.len() < 3 {
                println!("Error: migration name is required.");
                return Ok(());
            }
            let name = &args[2];
            create_migration_files(name)?;
        }
        "migrate" | "db:migrate" => {
            run_migrations(migrations).await?;
        }
        "migrate:rollback" | "db:rollback" => {
            rollback_migrations(migrations).await?;
        }
        "status" | "db:status" => {
            status_migrations(migrations).await?;
        }
        "db:seed" => {
            println!("Seeding database...");
            crate::Eloquent::seed(seeders).await?;
            println!("Database seeded successfully!");
        }
        _ => {
            println!("Unknown command: {}", command);
        }
    }
    Ok(())
}

pub async fn run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn crate::Seeder>>
) -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    run_artisan_with_args(&args, migrations, seeders).await
}

async fn status_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Eloquent::pool();
    let driver = crate::Eloquent::driver();

    let table_exists = match driver {
        "postgres" | "mysql" => {
            let query_str = "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await.unwrap_or((0,));
            row.0 > 0
        }
        _ => {
            let query_str = "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await.unwrap_or((0,));
            row.0 > 0
        }
    };

    let executed_set = if table_exists {
        let executed: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations")
            .fetch_all(pool)
            .await?;
        executed.into_iter().map(|(m,)| m).collect::<std::collections::HashSet<String>>()
    } else {
        std::collections::HashSet::new()
    };

    println!("{:<40} | {}", "Migration Name", "Status");
    println!("{}", "-".repeat(55));
    for m in migrations {
        let name = m.name();
        let status = if executed_set.contains(name) {
            "Applied"
        } else {
            "Pending"
        };
        println!("{:<40} | {}", name, status);
    }

    Ok(())
}

fn create_migration_files(name: &str) -> Result<(), Error> {
    use std::fs;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let snake_name = name.to_lowercase().replace("-", "_");
    let file_name = format!("m{}_{}", now, snake_name);
    
    fs::create_dir_all("src/migrations").map_err(|e| {
        Error::Protocol(format!("Failed to create migrations directory: {}", e))
    })?;

    let new_file_path = format!("src/migrations/{}.rs", file_name);
    let migration_code = format!(
r#"use rust_eloquent::schema::{{Schema, Blueprint, Migration}};
use rust_eloquent::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "m{timestamp}_{name}"
    }}

    async fn up(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::create("{name}", |table| {{
            table.id();
            // table.string("column_name");
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::drop_if_exists("{name}").await
    }}
}}
"#,
        timestamp = now,
        name = snake_name
    );

    fs::write(&new_file_path, migration_code).map_err(|e| {
        Error::Protocol(format!("Failed to write migration file: {}", e))
    })?;
    println!("Created migration file: {}", new_file_path);

    regenerate_migrations_mod()?;

    Ok(())
}

fn regenerate_migrations_mod() -> Result<(), Error> {
    use std::fs;
    let paths = fs::read_dir("src/migrations").map_err(|e| {
        Error::Protocol(format!("Failed to read migrations dir: {}", e))
    })?;

    let mut modules = vec![];
    for path in paths {
        let path = path.map_err(|e| Error::Protocol(e.to_string()))?.path();
        if let Some(ext) = path.extension() {
            if ext == "rs" {
                if let Some(stem) = path.file_stem() {
                    let stem_str = stem.to_string_lossy().to_string();
                    if stem_str != "mod" && stem_str.starts_with('m') {
                        modules.push(stem_str);
                    }
                }
            }
        }
    }
    modules.sort();

    let mut mod_content = String::new();
    mod_content.push_str("// Generated by Rust Eloquent Artisan. Do not edit manually.\n\n");
    for m in &modules {
        mod_content.push_str(&format!("pub mod {};\n", m));
    }
    mod_content.push_str("\npub fn get_migrations() -> Vec<Box<dyn rust_eloquent::schema::Migration>> {\n");
    mod_content.push_str("    vec![\n");
    for m in &modules {
        mod_content.push_str(&format!("        Box::new({}::MigrationImpl),\n", m));
    }
    mod_content.push_str("    ]\n");
    mod_content.push_str("}\n");

    fs::write("src/migrations/mod.rs", mod_content).map_err(|e| {
        Error::Protocol(format!("Failed to write mod.rs: {}", e))
    })?;
    println!("Regenerated src/migrations/mod.rs");

    Ok(())
}

async fn run_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Eloquent::pool();
    let driver = crate::Eloquent::driver();

    let query_str = match driver {
        "postgres" => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id SERIAL PRIMARY KEY,
                migration VARCHAR(255) NOT NULL,
                batch INTEGER NOT NULL
            )"
        }
        "mysql" => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id INT AUTO_INCREMENT PRIMARY KEY,
                migration VARCHAR(255) NOT NULL,
                batch INT NOT NULL
            )"
        }
        _ => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration TEXT NOT NULL,
                batch INTEGER NOT NULL
            )"
        }
    };

    sqlx::query(query_str).execute(pool).await?;

    let executed: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations")
        .fetch_all(pool)
        .await?;
    let executed_set: std::collections::HashSet<String> = executed.into_iter().map(|(m,)| m).collect();

    let batch_row: (Option<i32>,) = sqlx::query_as("SELECT MAX(batch) FROM migrations")
        .fetch_one(pool)
        .await?;
    let next_batch = batch_row.0.unwrap_or(0) + 1;

    let mut count = 0;
    for m in migrations {
        let name = m.name();
        if !executed_set.contains(name) {
            println!("Migrating: {}", name);
            m.up().await?;
            sqlx::query("INSERT INTO migrations (migration, batch) VALUES (?, ?)")
                .bind(name)
                .bind(next_batch)
                .execute(pool)
                .await?;
            println!("Migrated:  {}", name);
            count += 1;
        }
    }

    if count == 0 {
        println!("Nothing to migrate.");
    }

    Ok(())
}

async fn rollback_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Eloquent::pool();
    let driver = crate::Eloquent::driver();

    let table_exists = match driver {
        "postgres" | "mysql" => {
            let query_str = "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await.unwrap_or((0,));
            row.0 > 0
        }
        _ => {
            let query_str = "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await.unwrap_or((0,));
            row.0 > 0
        }
    };

    if !table_exists {
        println!("Nothing to rollback.");
        return Ok(());
    }

    let batch_row: (Option<i32>,) = sqlx::query_as("SELECT MAX(batch) FROM migrations")
        .fetch_one(pool)
        .await?;
    
    let last_batch = match batch_row.0 {
        Some(b) if b > 0 => b,
        _ => {
            println!("Nothing to rollback.");
            return Ok(());
        }
    };

    let to_rollback: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations WHERE batch = ? ORDER BY id DESC")
        .bind(last_batch)
        .fetch_all(pool)
        .await?;

    let mut rollback_map = std::collections::HashMap::new();
    for m in migrations {
        rollback_map.insert(m.name().to_string(), m);
    }

    for (name,) in to_rollback {
        if let Some(m) = rollback_map.get(&name) {
            println!("Rolling back: {}", name);
            m.down().await?;
            sqlx::query("DELETE FROM migrations WHERE migration = ?")
                .bind(&name)
                .execute(pool)
                .await?;
            println!("Rolled back:  {}", name);
        } else {
            println!("Warning: migration {} found in database but not in compiled binary.", name);
        }
    }

    Ok(())
}

pub struct JoinClause {
    pub table: String,
    pub conditions: Vec<String>,
    pub bindings: Vec<crate::EloquentValue>,
}

impl JoinClause {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            conditions: vec![],
            bindings: vec![],
        }
    }

    pub fn on(&mut self, first: &str, operator: &str, second: &str) -> &mut Self {
        self.conditions.push(format!("{} {} {}", first, operator, second));
        self
    }

    pub fn on_eq<T: Into<crate::EloquentValue>>(&mut self, column: &str, value: T) -> &mut Self {
        self.conditions.push(format!("{} = ?", column));
        self.bindings.push(value.into());
        self
    }
}

pub trait SubqueryBuilder {
    fn to_sql(&self) -> String;
    fn bindings(&self) -> &Vec<crate::EloquentValue>;
}

pub static QUERY_LOGGING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn enable_query_log() {
    QUERY_LOGGING.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn disable_query_log() {
    QUERY_LOGGING.store(false, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_query_log_enabled() -> bool {
    QUERY_LOGGING.load(std::sync::atomic::Ordering::SeqCst)
}
