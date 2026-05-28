use rust_eloquent::{Eloquent, sqlx::FromRow};

#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::main]
async fn main() -> Result<(), rust_eloquent::sqlx::Error> {
    // 1. Setup Primary and Replica databases
    let _ = std::fs::remove_file("primary.db");
    let _ = std::fs::remove_file("replica1.db");
    let _ = std::fs::remove_file("replica2.db");

    std::fs::File::create("primary.db").unwrap();
    std::fs::File::create("replica1.db").unwrap();
    std::fs::File::create("replica2.db").unwrap();

    // 2. Initialize Eloquent with 1 Primary and 2 Replicas
    Eloquent::init_with_replicas(
        "sqlite://primary.db",
        vec!["sqlite://replica1.db", "sqlite://replica2.db"]
    ).await?;

    // Create the users table on primary and both replicas (in a real-world scenario, replication is handled by the database engine)
    let primary_pool = Eloquent::pool();
    let r1_pool = rust_eloquent::EloquentPool::connect("sqlite://replica1.db").await?;
    let r2_pool = rust_eloquent::EloquentPool::connect("sqlite://replica2.db").await?;

    for pool in &[primary_pool, &r1_pool, &r2_pool] {
        rust_eloquent::sqlx::query(
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL)"
        )
        .execute(*pool)
        .await?;
    }

    println!("✅ Read/Write Connection Split initialized successfully!");
    
    // 3. Write data strictly routes to the primary pool
    println!("\n📥 Inserting users (routes to primary database pool)...");
    for i in 1..=10 {
        let mut user = User {
            id: 0,
            name: format!("User {}", i),
            email: format!("user{}@cosmos.com", i),
        };
        user.save().await?;
    }

    // In our manual setup, to simulate replication, let's copy the records from primary to the replica databases
    let all_users = rust_eloquent::sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(primary_pool)
        .await?;
    for user in all_users {
        for pool in &[&r1_pool, &r2_pool] {
            rust_eloquent::sqlx::query("INSERT INTO users (id, name, email) VALUES (?, ?, ?)")
                .bind(user.id)
                .bind(&user.name)
                .bind(&user.email)
                .execute(*pool)
                .await?;
        }
    }

    // Enable query logging to visualize connection/query details
    Eloquent::enable_query_log();

    // 4. Read operations route to replica pools round-robin
    println!("\n🔍 Running multiple read operations (load-balanced round-robin across replicas)...");
    let count1 = User::query().count().await?;
    let count2 = User::query().count().await?;
    println!("=> Count query 1: {}, Count query 2: {}", count1, count2);

    // 5. Query Chunking: low memory batch processing
    println!("\n📦 Testing Query Chunking (processing users in batches of 3)...");
    
    User::query()
        .chunk(3, |chunk| async move {
            println!("--- Processing a chunk of {} users ---", chunk.len());
            for user in chunk {
                println!("  - [{}] {} ({})", user.id, user.name, user.email);
            }
        })
        .await?;

    // Cleanup files
    let _ = std::fs::remove_file("primary.db");
    let _ = std::fs::remove_file("replica1.db");
    let _ = std::fs::remove_file("replica2.db");

    println!("\n🎉 Enterprise Scaling demo completed successfully!");
    Ok(())
}
