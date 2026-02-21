use crate::challenge::Base64Challenger;
use crate::domain::{SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::models::{NewSubscriber, Subscription, User};
use crate::startup::{ApplicationClientBaseUrl, CaptchaSecret};
use crate::utils::{e400, e404, e500};
use actix_web::{HttpResponse, post, web};
use anyhow::Context;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Deserialize, Serialize)]
pub struct SubscribeParams {
    answer_attempt: String,
    email: String,
    name: String,
    signed_answer: String,
    username: String,
}

impl NewSubscriber {
    async fn try_from(
        params: SubscribeParams,
        captcha_secret: SecretString,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Self, actix_web::Error> {
        let decrypted =
            Base64Challenger::decrypt(&params.signed_answer, captcha_secret).map_err(e400)?;

        if decrypted != params.answer_attempt {
            return Err(e400("Incorrect captcha answer."));
        }

        let name = SubscriberName::parse(params.name).map_err(e400)?;
        let email = SubscriberEmail::parse(params.email).map_err(e400)?;
        let user: User = User::find_by_username(&params.username, transaction)
            .await
            .context("Failed to find user.")
            .map_err(e404)?;

        Ok(Self {
            email,
            name,
            user_id: user.user_id,
        })
    }
}

#[post("/subscriptions")]
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(params, pool, email_client, base_url, captcha_secret),
    fields(
        subscriber_email = %params.email,
        subscriber_name = %params.name
    )
)]
pub async fn post(
    base_url: web::Data<ApplicationClientBaseUrl>,
    captcha_secret: web::Data<CaptchaSecret>,
    email_client: web::Data<EmailClient>,
    params: web::Json<SubscribeParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")
        .map_err(e500)?;
    let new_subscriber =
        NewSubscriber::try_from(params.0, captcha_secret.0.clone(), &mut transaction).await?;
    let subscription_token = Subscription::generate_subscription_token();
    let subscription = new_subscriber
        .insert_subscriber(&mut transaction)
        .await
        .context("Failed to insert new subscriber in the database.")
        .map_err(e500)?
        .store_token(&mut transaction, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")
        .map_err(e500)?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")
        .map_err(e500)?;
    subscription
        .send_confirmation_email(&email_client, &base_url.0, &subscription_token)
        .await
        .context("Failed to send a confirmation email.")
        .map_err(e500)?;

    Ok(HttpResponse::Ok().finish())
}
