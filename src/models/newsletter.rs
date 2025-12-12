use crate::domain::newsletter_issue::{Description, Title};
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, PgPool, Postgres, Transaction, postgres::PgRow};
use uuid::Uuid;
use voca_rs::strip;

#[derive(Serialize, Deserialize, Debug)]
pub struct NewsletterIssue {
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub newsletter_issue_id: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
    pub title: String,
    pub user_id: Uuid,
}

impl NewsletterIssue {
    pub async fn find_by_newsletter_issue_id(
        newsletter_issue_id: Uuid,
        db_pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let issue = sqlx::query_as!(
            NewsletterIssue,
            r#"
              SELECT
                content,
                created_at,
                description,
                newsletter_issue_id,
                published_at,
                slug,
                title,
                user_id
              FROM newsletter_issues
              WHERE
                newsletter_issue_id = $1
            "#,
            newsletter_issue_id
        )
        .fetch_one(db_pool)
        .await?;

        Ok(issue)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewsletterIssueEmail {
    pub description: String,
    pub html_content: String,
    pub newsletter_issue_id: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
    pub text_content: String,
    pub title: String,
    pub user_id: Uuid,
}

impl From<NewsletterIssue> for NewsletterIssueEmail {
    fn from(newsletter_issue: NewsletterIssue) -> NewsletterIssueEmail {
        let html_content = markdown::to_html(&newsletter_issue.content);
        let text_content = strip::strip_tags(&html_content);

        NewsletterIssueEmail {
            description: newsletter_issue.description,
            html_content,
            newsletter_issue_id: newsletter_issue.newsletter_issue_id,
            published_at: newsletter_issue.published_at,
            slug: newsletter_issue.slug,
            text_content,
            title: newsletter_issue.title,
            user_id: newsletter_issue.user_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewNewsletterIssue {
    pub content: String,
    pub description: String,
    pub newsletter_issue_id: Uuid,
    pub slug: String,
    pub title: String,
}

impl NewNewsletterIssue {
    pub async fn validate(
        self,
        user_id: &Uuid,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Self, anyhow::Error> {
        let user_id_and_slug_is_unique = self
            .validate_user_id_and_slug_uniqueness(user_id, transaction)
            .await
            .context("Failed to validate newsletter issue user id and slug uniqueness")?
            .is_none();

        if user_id_and_slug_is_unique {
            Ok(self)
        } else {
            anyhow::bail!("User id and slug are not unique.")
        }
    }

    async fn validate_user_id_and_slug_uniqueness(
        &self,
        user_id: &Uuid,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<PgRow>, sqlx::Error> {
        let result = transaction
            .fetch_optional(sqlx::query!(
                r#"
                  SELECT user_id, slug
                  FROM newsletter_issues
                  WHERE user_id = $1 AND slug = $2
                "#,
                user_id,
                self.slug
            ))
            .await?;

        Ok(result)
    }

    pub async fn insert_newsletter_issue(
        &self,
        user_id: &Uuid,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Uuid, sqlx::Error> {
        transaction
            .execute(sqlx::query!(
                r#"
                  INSERT INTO newsletter_issues (
                    content,
                    created_at,
                    description,
                    newsletter_issue_id,
                    published_at,
                    slug,
                    title,
                    user_id
                  )
                  VALUES ($1, now(), $2, $3, now(), $4, $5, $6)
                "#,
                self.content,
                self.description,
                self.newsletter_issue_id,
                self.slug,
                self.title,
                user_id,
            ))
            .await?;

        Ok(self.newsletter_issue_id)
    }
}

#[derive(Deserialize)]
pub struct NewNewsletterIssueData {
    pub content: String,
    pub description: String,
    pub title: String,
}

impl TryFrom<NewNewsletterIssueData> for NewNewsletterIssue {
    type Error = String;

    fn try_from(data: NewNewsletterIssueData) -> Result<NewNewsletterIssue, String> {
        let description = Description::parse(data.description)?;
        let newsletter_issue_id = Uuid::new_v4();
        let title = Title::parse(data.title)?;

        Ok(NewNewsletterIssue {
            content: data.content,
            description: description.as_ref().to_string(),
            newsletter_issue_id,
            slug: slug::slugify(title.as_ref()),
            title: title.as_ref().to_string(),
        })
    }
}
