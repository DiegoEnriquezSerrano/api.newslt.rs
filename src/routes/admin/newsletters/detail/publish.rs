use crate::authentication::UserId;
use crate::idempotency::{IdempotencyKey, NextAction, save_response, try_processing};
use crate::models::NewsletterIssue;
use crate::utils::{ResponseMessage, e400, e404, e500};
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, put, web};
use anyhow::Context;
use serde::Deserialize;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

const SUCCESS_MESSAGE: &str =
    "The newsletter issue has been accepted - emails will go out shortly.";

#[derive(Deserialize)]
struct PublishParams {
    idempotency_key: String,
}

#[put("/newsletter/{newsletter_issue_id}/publish")]
#[tracing::instrument(
  name = "Publish a newsletter issue",
  skip_all,
  fields(user_id=%*user_id)
)]
pub async fn put(
    params: web::Json<PublishParams>,
    path: web::Path<(Uuid,)>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let newsletter_issue_id = path.into_inner().0;
    let PublishParams { idempotency_key } = params.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok(saved_response);
        }
    };
    NewsletterIssue::find_by_user_id_and_newsletter_issue_id_txn(
        *user_id,
        &newsletter_issue_id,
        &mut transaction,
    )
    .await
    .context("Failed to query for newsletter issue.")
    .map_err(e404)?
    .validate_for_publish()
    .map_err(e400)?
    .publish_newsletter(&mut transaction)
    .await
    .context("Failed to publish newsletter issue details.")
    .map_err(e500)?;
    enqueue_delivery_tasks(&mut transaction, newsletter_issue_id, &user_id)
        .await
        .context("Failed to enqueue delivery tasks.")
        .map_err(e500)?;
    let response = HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(ResponseMessage::from(SUCCESS_MESSAGE));
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
    user_id: &Uuid,
) -> Result<(), sqlx::Error> {
    transaction
        .execute(sqlx::query!(
            r#"
              INSERT INTO issue_delivery_queue (
                newsletter_issue_id,
                subscriber_email
              )
              SELECT $1, email
              FROM subscriptions
              WHERE status = 'confirmed'
              AND user_id = $2
            "#,
            newsletter_issue_id,
            user_id
        ))
        .await?;

    Ok(())
}
