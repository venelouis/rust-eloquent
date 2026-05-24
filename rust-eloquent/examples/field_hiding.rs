use rust_eloquent::sqlx::FromRow;

#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[eloquent(hidden)]
    pub password: String,
}

#[tokio::main]
async fn main() -> Result<(), rust_eloquent::sqlx::Error> {
    println!("--- Model Serialization & Hiding Demo ---");

    // Initialize mock user
    let user = User {
        id: 42,
        name: "Arthur Dent".to_string(),
        email: "arthur@galaxy.guide".to_string(),
        password: "super_secret_password_123".to_string(),
    };

    println!("\nOriginal Struct representation (Debug):");
    println!("{:?}", user);

    println!("\nSerializing to JSON via user.to_json():");
    let json_str = user.to_json();
    println!("{}", json_str);

    // Verify properties
    assert!(json_str.contains("\"id\":42"));
    assert!(json_str.contains("\"name\":\"Arthur Dent\""));
    assert!(json_str.contains("\"email\":\"arthur@galaxy.guide\""));
    assert!(!json_str.contains("password"));
    assert!(!json_str.contains("super_secret_password_123"));

    println!("\n✅ Serialization verified! 'password' was successfully hidden.");
    Ok(())
}
