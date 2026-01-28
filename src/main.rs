#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, FromRow, SqlitePool};

#[derive(Debug, Serialize, FromRow)]
struct User {
    id: i64,
    username: String,
    email: String,
}

#[get("/users/<id>")]
async fn get_user_by_id(
    id: i64,
    db: &State<SqlitePool>,
) -> Result<Json<User>, Status> {
    // Use bind parameters to avoid SQL injection
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| {
        error!("db error in get_user_by_id({}): {}", id, e);
        Status::InternalServerError
    })?;

    match user {
        Some(u) => Ok(Json(u)),
        None => Err(Status::NotFound),
    }
}

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
        .mount("/", routes![get_user_by_id])
}
