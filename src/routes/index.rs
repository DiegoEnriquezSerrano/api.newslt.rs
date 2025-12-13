use crate::utils::ResponseMessage;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get};

#[get("/")]
pub async fn get() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(ResponseMessage::from("Welcome to our newsletter!"))
}
