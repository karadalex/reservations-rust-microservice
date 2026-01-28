use rocket::http::Status;
use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub message: String,
}

pub type ApiResult<T> = Result<Json<T>, (Status, Json<ErrorBody>)>;

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
