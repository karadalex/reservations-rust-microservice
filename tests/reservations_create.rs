use reservations_rust_microservice::reservations;
use reservations_rust_microservice::utils::issue_jwt;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use sqlx::sqlite::SqlitePoolOptions;

async fn build_rocket() -> rocket::Rocket<rocket::Build> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory SQLite pool");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS reservations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            start_datetime TEXT NOT NULL,
            end_datetime TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create reservations table");

    rocket::build().manage(pool).mount("/", reservations::routes())
}

#[rocket::async_test]
async fn create_reservation_returns_created_row() {
    std::env::set_var("JWT_SECRET", "test-secret");
    let token = issue_jwt(1).expect("issue token");

    let client = Client::tracked(build_rocket().await)
        .await
        .expect("valid rocket instance");

    let response = client
        .post("/reservations")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .body(
            r#"{"user_id":1,"start_datetime":"2026-01-28T10:00:00Z","end_datetime":"2026-01-28T11:00:00Z"}"#,
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    let body: serde_json::Value = response
        .into_json()
        .await
        .expect("valid JSON response");

    assert!(body["id"].is_number());
    assert_eq!(body["user_id"], 1);
    assert_eq!(body["start_datetime"], "2026-01-28T10:00:00Z");
    assert_eq!(body["end_datetime"], "2026-01-28T11:00:00Z");
}
