use rocket::http::Status;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use rocket::{get, post, error};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

use crate::utils::*;
use crate::error_response;

pub fn routes() -> Vec<rocket::Route> {
    routes![get_user_by_id, create_user, login]
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct User {
    #[serde(default)]
    id: Option<i64>,
    username: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct NewUser {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
}

#[derive(Debug, FromRow)]
struct UserAuthRow {
    id: i64,
    password_hash: String,
}

fn hash_password(password: &str) -> Result<String, ErrorBody> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ErrorBody {
            message: "failed to hash password".to_string(),
        })
}

#[get("/users/<id>")]
async fn get_user_by_id(
    id: i64,
    db: &State<SqlitePool>,
    auth: AuthUser,
) -> ApiResult<User>  {
    if id != auth.user_id {
        return Err(error_response!(Status::Forbidden, "forbidden"));
    }

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
		error_response!(Status::InternalServerError, "failed to fetch user")
    })?;

    match user {
        Some(u) => Ok(Json(u)),
        None => Err(error_response!(Status::NotFound, "user not found")),
    }
}

#[post("/users", data = "<new_user>")]
async fn create_user(
    new_user: Json<NewUser>,
    db: &State<SqlitePool>,
) -> ApiResult<User> {
    let password_hash = hash_password(&new_user.password)
        .map_err(|_| error_response!(Status::InternalServerError, "failed to hash password"))?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email, password_hash)
        VALUES (?, ?, ?)
        RETURNING id, username, email
        "#,
    )
    .bind(&new_user.username)
    .bind(&new_user.email)
    .bind(password_hash)
    .fetch_one(db.inner())
    .await
    .map_err(|e| {
        error!(
            "db error in create_user(username={}, email={}): {}",
            new_user.username, new_user.email, e
        );
		error_response!(Status::InternalServerError, "failed to create user")
    })?;

    Ok(Json(user))
}

#[post("/auth/login", data = "<login>")]
async fn login(
    login: Json<LoginRequest>,
    db: &State<SqlitePool>,
) -> ApiResult<LoginResponse> {
    let row = sqlx::query_as::<_, UserAuthRow>(
        r#"
        SELECT id, password_hash
        FROM users
        WHERE email = ?
        "#,
    )
    .bind(&login.email)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| {
        error!("db error in login(email={}): {}", login.email, e);
        error_response!(Status::InternalServerError, "failed to login")
    })?;

    let row = match row {
        Some(r) => r,
        None => return Err(error_response!(Status::Unauthorized, "invalid credentials")),
    };

    let parsed = PasswordHash::new(&row.password_hash)
        .map_err(|_| error_response!(Status::InternalServerError, "failed to verify password"))?;
    let argon2 = Argon2::default();
    if argon2
        .verify_password(login.password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(error_response!(Status::Unauthorized, "invalid credentials"));
    }

    let token = issue_jwt(row.id)
        .map_err(|e| (Status::InternalServerError, Json(e)))?;

    Ok(Json(LoginResponse { token }))
}
