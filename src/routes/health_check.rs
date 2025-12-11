use actix_web::{HttpResponse, get};

#[get("/health_check")]
pub async fn get() -> HttpResponse {
    HttpResponse::Ok().finish()
}
