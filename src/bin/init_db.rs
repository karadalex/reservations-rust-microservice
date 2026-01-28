use rocket_db_pools::sqlx::{self, sqlite::SqlitePoolOptions};

#[rocket::tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Default matches Rocket.toml (./database.sqlite) using SQLx URI form.
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://database.sqlite".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            email TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS reservations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            start_datetime TEXT NOT NULL,
            end_datetime TEXT NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(())
}
