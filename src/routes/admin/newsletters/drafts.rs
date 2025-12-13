use crate::authentication::UserId;
use crate::models::{NewsletterIssue, NewsletterIssueAPI};
use crate::utils::e500;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, web};
use anyhow::Context;
use sqlx::PgPool;

#[get("/newsletters/drafts")]
#[tracing::instrument(
    name = "Retrieving user's newsletter issues",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn get(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let newsletter_issues = NewsletterIssue::get_unpublished_by_user_id(*user_id, &pool)
        .await
        .context("Failed to query newsletter issues.")
        .map_err(e500)?;
    let mut newsletter_issues_api_vec: Vec<NewsletterIssueAPI> = vec![];

    for newsletter_issue in newsletter_issues {
        newsletter_issues_api_vec.push(NewsletterIssueAPI::from(newsletter_issue));
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(newsletter_issues_api_vec))
}
