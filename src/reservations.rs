use rocket::http::Status;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use rocket::{get, post, put, error};
use log::info;

use crate::utils::*;
use crate::error_response;


pub fn routes() -> Vec<rocket::Route> {
    routes![get_reservation_by_id, create_reservation, update_reservation]
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Reservation {
    #[serde(default)]
    id: Option<i64>,
    user_id: i64,
    start_datetime: String,
    end_datetime: String,
	is_active: Option<bool>,
	created_at: Option<String>,
	updated_at: Option<String>,
}

impl Reservation {
    async fn there_is_overlap_in_db(&self, db: &State<SqlitePool>) -> Result<bool, Status> {
        let there_is_overlap: bool = sqlx::query_scalar::<_, bool>(
        r#"
            SELECT EXISTS(
                SELECT 1 FROM reservations
                WHERE user_id = ?
				AND is_active = 1
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

	fn is_valid(&self) -> bool {
		self.start_datetime < self.end_datetime
	}
}


#[get("/reservations/<id>")]
async fn get_reservation_by_id(
    id: i64,
    db: &State<SqlitePool>,
    auth: AuthUser,
) -> ApiResult<Reservation> {
    // Use bind parameters to avoid SQL injection
    let reservation = sqlx::query_as::<_, Reservation>(
        r#"
        SELECT id, user_id, start_datetime, end_datetime, is_active, created_at, updated_at
        FROM reservations
        WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
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


#[put("/reservations/<id>", data = "<updated_reservation>")]
async fn update_reservation(
	id: i64,
	db: &State<SqlitePool>,
	updated_reservation: Json<Reservation>,
	auth: AuthUser,
) -> ApiResult<Reservation> {
	let reservation = sqlx::query_as::<_, Reservation>(
		r#"
		UPDATE reservations
		SET start_datetime = ?, end_datetime = ?, is_active = ?, updated_at = datetime('now')
		WHERE id = ? AND user_id = ?
		RETURNING id, user_id, start_datetime, end_datetime, is_active, created_at, updated_at
		"#,
	)
	.bind(&updated_reservation.start_datetime)
	.bind(&updated_reservation.end_datetime)
	.bind(updated_reservation.is_active.unwrap_or(true))
	.bind(id)
	.bind(auth.user_id)
	.fetch_one(db.inner())
	.await
	.map_err(|e| {
		error!(
			"db error in update_reservation(id={}, user_id={}): {}",
			id, auth.user_id, e
		);
		error_response!(Status::InternalServerError, "failed to update reservation")
	})?;

	Ok(Json(reservation))
}


#[post("/reservations", data = "<new_reservation>")]
async fn create_reservation(
    new_reservation: Json<Reservation>,
    db: &State<SqlitePool>,
    auth: AuthUser,
) -> ApiResult<Reservation> {
    if new_reservation.user_id != auth.user_id {
        return Err(error_response!(Status::Forbidden, "forbidden"));
    }

    let do_overlap = new_reservation
        .there_is_overlap_in_db(db)
        .await
		.map_err(|_| {
			error_response!(Status::InternalServerError, "failed to check reservation overlap")
        })?;
    if do_overlap {
		info!("Reservation overlap detected for user_id={}", new_reservation.user_id);
        return Err(error_response!(Status::Conflict, "reservation time overlaps with an existing reservation"));
    }
	if !new_reservation.is_valid() {
		return Err(error_response!(Status::BadRequest, "invalid reservation time range"));
	}

    let reservation = sqlx::query_as::<_, Reservation>(
        r#"
        INSERT INTO reservations (user_id, start_datetime, end_datetime, is_active, created_at, updated_at)
        VALUES (?, ?, ?, ?, COALESCE(?, datetime('now')), COALESCE(?, datetime('now')))
        RETURNING id, user_id, start_datetime, end_datetime, is_active, created_at, updated_at
        "#,
    )
    .bind(new_reservation.user_id)
    .bind(&new_reservation.start_datetime)
    .bind(&new_reservation.end_datetime)
    .bind(new_reservation.is_active.unwrap_or(true))
    .bind(new_reservation.created_at.as_deref())
    .bind(new_reservation.updated_at.as_deref())
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
