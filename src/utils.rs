use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub message: String,
}

pub type ApiResult<T> = Result<Json<T>, (Status, Json<ErrorBody>)>;

#[derive(Debug)]
pub struct AuthUser {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i64,
    exp: usize,
}

#[macro_export]
macro_rules! error_response {
    ($status:expr, $msg:expr) => {
        (
            $status,
            Json($crate::utils::ErrorBody {
                message: $msg.to_string(),
            }),
        )
    };
}

fn jwt_secret() -> Result<String, ErrorBody> {
    std::env::var("JWT_SECRET").map_err(|_| ErrorBody {
        message: "JWT_SECRET is not set".to_string(),
    })
}

fn jwt_expiration() -> usize {
    let ttl = std::env::var("JWT_TTL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(24 * 60 * 60);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (now + ttl) as usize
}

pub fn issue_jwt(user_id: i64) -> Result<String, ErrorBody> {
    let claims = Claims {
        sub: user_id,
        exp: jwt_expiration(),
    };
    let secret = jwt_secret()?;
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|_| ErrorBody {
            message: "failed to sign token".to_string(),
        })
}

pub fn parse(dt: &str) -> DateTime<Utc> {
    dt.parse::<DateTime<Utc>>().expect("invalid datetime")
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = Json<ErrorBody>;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth = req.headers().get_one("Authorization").unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if token.is_empty() {
            return Outcome::Error(error_response!(Status::Unauthorized, "missing bearer token"));
        }

        let secret = match jwt_secret() {
            Ok(v) => v,
            Err(e) => {
                return Outcome::Error(error_response!(Status::InternalServerError, "JWT secret not configured"));
            }
        };

        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| { error_response!(Status::Unauthorized, "invalid token") } );

        match data {
            Ok(d) => Outcome::Success(AuthUser { user_id: d.claims.sub }),
            Err(e) => Outcome::Error(e),
        }
    }
}
