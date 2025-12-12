use crate::authentication::UserId;
use crate::idempotency::{IdempotencyKey, NextAction, save_response, try_processing};
use crate::models::{NewNewsletterIssue, NewNewsletterIssueData};
use crate::utils::{e400, e500};
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, post, web};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use serde::Deserialize;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

const SUCCESS_MESSAGE: &str =
    "The newsletter issue has been accepted - emails will go out shortly.";

#[derive(Deserialize)]
pub struct PublishNewsletterParams {
    content: String,
    description: String,
    idempotency_key: String,
    title: String,
}

fn success_message() -> FlashMessage {
    FlashMessage::info(SUCCESS_MESSAGE)
}

#[post("/newsletters")]
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn post(
    params: web::Json<PublishNewsletterParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let new_newsletter_issue: NewNewsletterIssue = NewNewsletterIssueData {
        content: params.0.content,
        description: params.0.description,
        title: params.0.title,
    }
    .try_into()
    .map_err(e400)?;
    let idempotency_key: IdempotencyKey = params.0.idempotency_key.try_into().map_err(e400)?;
    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };
    let issue_id = new_newsletter_issue
        .validate(&user_id, &mut transaction)
        .await
        .map_err(e400)?
        .insert_newsletter_issue(&user_id, &mut transaction)
        .await
        .context("Failed to store newsletter issue details.")
        .map_err(e500)?;
    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;
    let response = HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(serde_json::json!({ "message": SUCCESS_MESSAGE }));
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    success_message().send();
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id, 
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    );
    transaction.execute(query).await?;
    Ok(())
}
