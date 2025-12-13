use crate::authentication::UserId;
use crate::models::{NewsletterIssue, NewsletterIssueAPI};
use crate::utils::e404;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, web};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[get("/newsletters/{newsletter_issue_id}")]
#[tracing::instrument(
    name = "Retrieving a user's newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn get(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    path: web::Path<(Uuid,)>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let newsletter_issue_id = path.into_inner().0;
    let newsletter_issue = NewsletterIssue::find_by_user_id_and_newsletter_issue_id(
        *user_id,
        &newsletter_issue_id,
        &pool,
    )
    .await
    .context("Failed to find newsletter issue.")
    .map_err(e404)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(NewsletterIssueAPI::from(newsletter_issue)))
}
