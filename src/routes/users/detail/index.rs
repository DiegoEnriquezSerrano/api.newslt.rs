use crate::models::{PublicProfile, UserProfile};
use crate::utils::e404;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, web};
use anyhow::Context;
use sqlx::PgPool;

#[get("/users/{username}")]
#[tracing::instrument(name = "Retrieving a user profile", skip(pool))]
pub async fn get(
    pool: web::Data<PgPool>,
    path: web::Path<(String,)>,
) -> Result<HttpResponse, actix_web::Error> {
    let params = path.into_inner();
    let username = params.0;
    let user_profile: PublicProfile = UserProfile::find_public_profile(username, &pool)
        .await
        .context("Failed to find user profile.")
        .map_err(e404)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(user_profile))
}
