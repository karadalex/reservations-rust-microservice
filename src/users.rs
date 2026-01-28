use rocket::http::Status;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use rocket::{get, post, error};

pub fn routes() -> Vec<rocket::Route> {
    routes![get_user_by_id, create_user]
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct User {
    #[serde(default)]
    id: Option<i64>,
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

#[post("/users", data = "<new_user>")]
async fn create_user(
    new_user: Json<User>,
    db: &State<SqlitePool>,
) -> Result<Json<User>, Status> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email)
        VALUES (?, ?)
        RETURNING id, username, email
        "#,
    )
    .bind(&new_user.username)
    .bind(&new_user.email)
    .fetch_one(db.inner())
    .await
    .map_err(|e| {
        error!(
            "db error in create_user(username={}, email={}): {}",
            new_user.username, new_user.email, e
        );
        Status::InternalServerError
    })?;

    Ok(Json(user))
}
