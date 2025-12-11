use actix_web::{HttpResponse, get};

#[get("/authenticate")]
#[tracing::instrument(name = "Authenticating user", skip_all)]
pub async fn get() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().finish())
}
