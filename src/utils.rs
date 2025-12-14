use actix_web::http::StatusCode;
use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

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

// Return a 404 with the user-representation of the validation error as body.
// The error root cause is preserved for logging purposes.
pub fn e404<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    ServerError::NotFoundError(e).into()
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
    #[error("{0}")]
    NotFoundError(T),
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
            ServerError::NotFoundError(_) => StatusCode::NOT_FOUND,
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

impl From<String> for ResponseErrorMessage {
    fn from(value: String) -> Self {
        Self { error: value }
    }
}

impl From<&str> for ResponseErrorMessage {
    fn from(value: &str) -> Self {
        Self {
            error: value.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseMessage {
    pub message: String,
}

impl From<String> for ResponseMessage {
    fn from(value: String) -> Self {
        Self { message: value }
    }
}

impl From<&str> for ResponseMessage {
    fn from(value: &str) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}

const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

pub fn contains_forbidden_characters(s: &str) -> bool {
    s.chars().any(|g| FORBIDDEN_CHARACTERS.contains(&g))
}

pub fn is_too_long(s: &str, max: usize) -> bool {
    s.graphemes(true).count() > max
}

pub fn is_empty_or_whitespace(s: &str) -> bool {
    s.trim().is_empty()
}
