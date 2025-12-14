use crate::authentication::UserId;
use crate::domain::newsletter_issue::{Content, Description, Title};
use crate::models::{NewsletterIssue, NewsletterIssueAPI};
use crate::utils::{e400, e404, e500};
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, put, web};
use anyhow::Context;
use serde::Deserialize;
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

#[derive(Deserialize)]
pub struct NewsletterIssueUpdateParams {
    content: String,
    description: String,
    title: String,
}

impl NewsletterIssue {
    fn validate_update(
        mut self,
        data: NewsletterIssueUpdateParams,
    ) -> Result<NewsletterIssue, String> {
        let mut content = self.content;
        let description: Description;
        let title = Title::parse(data.title)?;

        if self.published_at.is_none() {
            description = Description::parse_draft(data.description)?;
        } else {
            description = Description::parse(data.description)?;
            content = Content::parse(data.content)?.as_ref().to_string();
        }

        self.content = content;
        self.description = description.as_ref().to_string();
        self.title = title.as_ref().to_string();

        Ok(self)
    }
}

#[put("/newsletters/{newsletter_issue_id}")]
#[tracing::instrument(
  name = "Update a newsletter issue",
  skip_all,
  fields(user_id=%*user_id)
)]
pub async fn put(
    params: web::Json<NewsletterIssueUpdateParams>,
    path: web::Path<(Uuid,)>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let newsletter_issue_id = path.into_inner().0;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to begin database transaction.")
        .map_err(e500)?;
    let newsletter_issue_api: NewsletterIssueAPI =
        NewsletterIssue::find_by_user_id_and_newsletter_issue_id_txn(
            *user_id,
            &newsletter_issue_id,
            &mut transaction,
        )
        .await
        .map_err(e404)?
        .validate_update(params.0)
        .map_err(e400)?
        .update(&mut transaction)
        .await
        .context("Failed to update newsletter issue.")
        .map_err(e500)?
        .into();
    transaction
        .commit()
        .await
        .context("Failed to commit transaction.")
        .map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(newsletter_issue_api))
}
