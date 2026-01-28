use rocket::http::Status;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use rocket::{get, post, error};

pub fn routes() -> Vec<rocket::Route> {
    routes![get_reservation_by_id, create_reservation]
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Reservation {
    #[serde(default)]
    id: Option<i64>,
    user_id: i64,
    start_datetime: String,
    end_datetime: String,
}


#[get("/reservations/<id>")]
async fn get_reservation_by_id(
    id: i64,
    db: &State<SqlitePool>,
) -> Result<Json<Reservation>, Status> {
    // Use bind parameters to avoid SQL injection
    let reservation = sqlx::query_as::<_, Reservation>(
        r#"
        SELECT id, user_id, start_datetime, end_datetime
        FROM reservations
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| {
        error!("db error in get_reservation_by_id({}): {}", id, e);
        Status::InternalServerError
    })?;

    match reservation {
        Some(r) => Ok(Json(r)),
        None => Err(Status::NotFound),
    }
}

#[post("/reservations", data = "<new_reservation>")]
async fn create_reservation(
    new_reservation: Json<Reservation>,
    db: &State<SqlitePool>,
) -> Result<Json<Reservation>, Status> {
    let reservation = sqlx::query_as::<_, Reservation>(
        r#"
        INSERT INTO reservations (user_id, start_datetime, end_datetime)
        VALUES (?, ?, ?)
        RETURNING id, user_id, start_datetime, end_datetime
        "#,
    )
    .bind(new_reservation.user_id)
    .bind(&new_reservation.start_datetime)
    .bind(&new_reservation.end_datetime)
    .fetch_one(db.inner())
    .await
    .map_err(|e| {
        error!(
            "db error in create_reservation(user_id={}, start_datetime={}, end_datetime={}): {}",
            new_reservation.user_id, new_reservation.start_datetime, new_reservation.end_datetime, e
        );
        Status::InternalServerError
    })?;

    Ok(Json(reservation))
}