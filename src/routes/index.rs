use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get};

#[get("/")]
pub async fn get() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(serde_json::json!({"message": "Welcome to our newsletter!"}))
}
