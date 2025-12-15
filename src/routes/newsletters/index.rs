use crate::models::{NewsletterIssue, PublicNewsletter};
use crate::utils::e500;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, web};
use anyhow::Context;
use sqlx::PgPool;

#[get("/newsletters")]
#[tracing::instrument(name = "Retrieving list of published newsletters", skip_all)]
pub async fn get(pool: web::Data<PgPool>) -> Result<HttpResponse, actix_web::Error> {
    let newsletter_issues: Vec<PublicNewsletter> = NewsletterIssue::get_public_newsletters(&pool)
        .await
        .context("Failed to query for newsletter issues.")
        .map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(newsletter_issues))
}
