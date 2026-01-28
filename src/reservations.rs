use rocket::http::Status;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use rocket::{get, post, error};


use crate::utils::*;
use crate::error_response;

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

impl Reservation {
    fn new(user_id: i64, start_datetime: String, end_datetime: String) -> Self {
		Reservation {
			id: None,
			user_id,
			start_datetime,
			end_datetime,
		}
	}

    async fn there_is_overlap_in_db(&self, db: &State<SqlitePool>) -> Result<bool, Status> {
        let there_is_overlap: bool = sqlx::query_scalar::<_, bool>(
        r#"
            SELECT EXISTS(
                SELECT 1 FROM reservations
                WHERE user_id = ?
                AND (
                    (start_datetime < ? AND end_datetime > ?)
                    OR (start_datetime < ? AND end_datetime < ?)
                    OR (start_datetime > ? AND end_datetime > ?)
                )
            );
            "#,
        )
        .bind(self.user_id)
        .bind(&self.end_datetime)
        .bind(&self.start_datetime)
        .bind(&self.start_datetime)
        .bind(&self.end_datetime)
        .bind(&self.start_datetime)
        .bind(&self.end_datetime)
        .fetch_one(db.inner())
        .await
        .map_err(|e| {
            error!(
                "db error in there_is_overlap_in_db(user_id={}): {}",
                self.user_id, e
            );
            Status::InternalServerError
        })?;

        Ok(there_is_overlap)
    }

	fn there_is_overlap(&self, other: Reservation) -> bool {
		(self.start_datetime < other.end_datetime &&
		self.end_datetime > other.start_datetime) || 
		(self.start_datetime < other.start_datetime &&
		self.end_datetime < other.end_datetime) || 
		(self.start_datetime > other.start_datetime &&
		self.end_datetime > other.end_datetime)
	}
}

#[get("/reservations/<id>")]
async fn get_reservation_by_id(
    id: i64,
    db: &State<SqlitePool>,
) -> ApiResult<Reservation> {
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
		error_response!(Status::InternalServerError, "failed to fetch reservation")
    })?;

    match reservation {
        Some(r) => Ok(Json(r)),
        None => Err(error_response!(Status::NotFound, "reservation not found")),
    }
}

#[post("/reservations", data = "<new_reservation>")]
async fn create_reservation(
    new_reservation: Json<Reservation>,
    db: &State<SqlitePool>,
) -> ApiResult<Reservation> {

    let do_overlap = new_reservation
        .there_is_overlap_in_db(db)
        .await
		.map_err(|_| {
			error_response!(Status::InternalServerError, "failed to check reservation overlap")
        })?;
    if do_overlap {
        return Err(error_response!(Status::Conflict, "reservation time overlaps with an existing reservation"));
    }

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
		error_response!(Status::InternalServerError, "failed to create reservation")
    })?;

    Ok(Json(reservation))
}
