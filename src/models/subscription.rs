use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;
use crate::email_client::EmailClient;
use crate::utils::error_chain_fmt;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use serde::Serialize;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct Subscription {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub status: String,
}

impl Subscription {
    pub fn generate_subscription_token() -> String {
        let mut rng = thread_rng();
        std::iter::repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(25)
            .collect()
    }

    pub async fn send_confirmation_email(
        &self,
        email_client: &EmailClient,
        base_url: &str,
        subscription_token: &str,
    ) -> Result<(), reqwest::Error> {
        let confirmation_link = format!(
            "{}/subscriptions/confirm?subscription_token={}",
            base_url, subscription_token
        );
        let plain_body = format!(
            "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
            confirmation_link
        );
        let html_body = format!(
            "Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.",
            confirmation_link
        );
        email_client
            .send_email(&self.email, "Welcome!", &html_body, &plain_body)
            .await
    }

    pub async fn store_token(
        self,
        transaction: &mut Transaction<'_, Postgres>,
        subscription_token: &str,
    ) -> Result<Self, StoreTokenError> {
        transaction
            .execute(sqlx::query!(
                r#"
                  INSERT INTO subscription_tokens (subscription_token, subscriber_id)
                  VALUES ($1, $2)
                "#,
                subscription_token,
                &self.id
            ))
            .await
            .map_err(StoreTokenError)?;

        Ok(self)
    }
}

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub user_id: Uuid,
}

impl NewSubscriber {
    pub async fn insert_subscriber(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Subscription, sqlx::Error> {
        let subscriber_id = Uuid::new_v4();
        let status = SubscriptionStatus::PendingConfirmation.as_str();
        transaction
            .execute(sqlx::query!(
                r#"
                  INSERT INTO subscriptions (
                    id,
                    email,
                    name,
                    subscribed_at,
                    status,
                    user_id
                  )
                  VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                &subscriber_id,
                self.email.as_ref(),
                self.name.as_ref(),
                Utc::now(),
                status,
                &self.user_id
            ))
            .await?;

        Ok(Subscription {
            email: self.email.to_string(),
            id: subscriber_id,
            name: self.name.as_ref().to_string(),
            status: status.to_string(),
        })
    }
}

pub struct StoreTokenError(sqlx::Error);

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database failure was encountered while trying to store a subscription token."
        )
    }
}

pub enum SubscriptionStatus {
    PendingConfirmation,
    Confirmed,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::Confirmed => "confirmed",
            SubscriptionStatus::PendingConfirmation => "pending_confirmation",
        }
    }
}
