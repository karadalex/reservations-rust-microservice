#[macro_use]
extern crate rocket;

mod users;
mod reservations;
mod utils;
use sqlx::sqlite::SqlitePoolOptions;


#[launch]
async fn rocket() -> _ {
    // For local dev this could be e.g. "sqlite://app.db"
    // Or "sqlite::memory:" for in-memory testing.
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://database.sqlite".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap_or_else(|e| {
            error!("failed to connect to SQLite at {}: {}", database_url, e);
            panic!("failed to connect to SQLite");
        });

    rocket::build()
        .manage(pool)
        .mount("/", users::routes())
        .mount("/", reservations::routes())
}
