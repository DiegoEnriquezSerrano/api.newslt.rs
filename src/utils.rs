use crate::routes::error_chain_fmt;
use actix_web::http::StatusCode;
use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};

// Return an opaque 500 while preserving the error root's cause for logging.
pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    ServerError::UnexpectedError(e).into()
}

// Return a 400 with the user-representation of the validation error as body.
// The error root cause is preserved for logging purposes.
pub fn e400<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    ServerError::BadRequestError(e).into()
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}

#[derive(thiserror::Error)]
pub enum ServerError<T: std::fmt::Debug + std::fmt::Display + 'static> {
    #[error("{0}")]
    UnexpectedError(T),
    #[error("{0}")]
    BadRequestError(T),
}

impl<T: std::fmt::Debug + std::fmt::Display + 'static> std::fmt::Debug for ServerError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl<T: std::fmt::Debug + std::fmt::Display + 'static> ResponseError for ServerError<T> {
    fn status_code(&self) -> StatusCode {
        match self {
            ServerError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::BadRequestError(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type(ContentType::json())
            .json(ResponseErrorMessage {
                error: format!("{}", self),
            })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseErrorMessage {
    pub error: String,
}
