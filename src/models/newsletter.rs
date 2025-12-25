use crate::clients::cloudinary_client::CloudinaryClient;
use crate::clients::s3_client::S3Client;
use crate::domain::newsletter_issue::{Content, Description, Title};
use crate::domain::{Base64ImageUrl, ImageUrl};
use crate::models::AssociatedUser;
use crate::utils::{e500, is_empty_or_whitespace};
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use sqlx::postgres::PgRow;
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use voca_rs::strip;

#[derive(Serialize, Deserialize, Debug)]
pub struct NewsletterIssue {
    pub content: String,
    pub cover_image_url: String,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub newsletter_issue_id: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
    pub title: String,
    pub user_id: Uuid,
}

impl TryFrom<PgRow> for NewsletterIssue {
    type Error = sqlx::Error;

    fn try_from(row: PgRow) -> Result<Self, Self::Error> {
        Ok(Self {
            content: row.try_get("content")?,
            cover_image_url: row.try_get("cover_image_url")?,
            created_at: row.try_get("created_at")?,
            description: row.try_get("description")?,
            newsletter_issue_id: row.try_get("newsletter_issue_id")?,
            published_at: row.try_get("published_at")?,
            slug: row.try_get("slug")?,
            title: row.try_get("title")?,
            user_id: row.try_get("user_id")?,
        })
    }
}

impl NewsletterIssue {
    pub async fn find_by_user_id_and_newsletter_issue_id(
        user_id: Uuid,
        newsletter_issue_id: &Uuid,
        pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let newsletter_issue = sqlx::query_as!(
            NewsletterIssue,
            r#"
              SELECT
                content,
                cover_image_url,
                created_at,
                description,
                newsletter_issue_id,
                published_at,
                slug,
                title,
                user_id
              FROM newsletter_issues
              WHERE user_id = $1 AND newsletter_issue_id = $2
              LIMIT 1
            "#,
            user_id,
            newsletter_issue_id
        )
        .fetch_one(pool)
        .await?;

        Ok(newsletter_issue)
    }

    pub async fn find_by_user_id_and_newsletter_issue_id_txn(
        user_id: Uuid,
        newsletter_issue_id: &Uuid,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Self, sqlx::Error> {
        let newsletter_issue: NewsletterIssue = transaction
            .fetch_one(sqlx::query_as!(
                NewsletterIssue,
                r#"
                  SELECT
                    content,
                    cover_image_url,
                    created_at,
                    description,
                    newsletter_issue_id,
                    published_at,
                    slug,
                    title,
                    user_id
                  FROM newsletter_issues
                  WHERE user_id = $1 AND newsletter_issue_id = $2
                  LIMIT 1
                "#,
                user_id,
                newsletter_issue_id
            ))
            .await?
            .try_into()?;

        Ok(newsletter_issue)
    }

    pub async fn find_by_newsletter_issue_id(
        newsletter_issue_id: Uuid,
        db_pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let issue = sqlx::query_as!(
            NewsletterIssue,
            r#"
              SELECT
                content,
                cover_image_url,
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

    pub async fn get_published_by_user_id(
        user_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let newsletter_issues = sqlx::query_as!(
            NewsletterIssue,
            r#"
              SELECT
                content,
                cover_image_url,
                created_at,
                description,
                newsletter_issue_id,
                published_at,
                slug,
                title,
                user_id
              FROM newsletter_issues
              WHERE user_id = $1 AND published_at IS NOT NULL
              ORDER BY published_at DESC
              LIMIT 10
            "#,
            user_id
        )
        .fetch_all(pool)
        .await?;

        Ok(newsletter_issues)
    }

    pub async fn get_unpublished_by_user_id(
        user_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let newsletter_issues = sqlx::query_as!(
            NewsletterIssue,
            r#"
              SELECT
                content,
                cover_image_url,
                created_at,
                description,
                newsletter_issue_id,
                published_at,
                slug,
                title,
                user_id
              FROM newsletter_issues
              WHERE user_id = $1 AND published_at IS NULL
              ORDER BY created_at DESC
              LIMIT 10
            "#,
            user_id
        )
        .fetch_all(pool)
        .await?;

        Ok(newsletter_issues)
    }

    pub async fn update(
        self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Self, sqlx::Error> {
        transaction
            .execute(sqlx::query!(
                r#"
                  UPDATE newsletter_issues
                  SET content = $1,
                      cover_image_url = $2,
                      description = $3,
                      title = $4
                  WHERE newsletter_issue_id = $5
                      AND user_id = $6
                "#,
                &self.content,
                &self.cover_image_url,
                &self.description,
                &self.title,
                &self.newsletter_issue_id,
                &self.user_id
            ))
            .await?;

        Ok(self)
    }

    pub async fn publish_newsletter(
        self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Uuid, sqlx::Error> {
        transaction
            .execute(sqlx::query!(
                r#"
                  UPDATE newsletter_issues
                  SET published_at = now()
                  WHERE newsletter_issue_id = $1
                    AND user_id = $2
                "#,
                &self.newsletter_issue_id,
                self.user_id,
            ))
            .await?;

        Ok(self.newsletter_issue_id)
    }

    // Note - For serializing nested records sqlx allows returning
    // sequence of values, which are then mapped to the key names in
    // order. This means the order of the columns in the query must
    // reflect the order in the struct definition. For casted values,
    // sqlx only checks that the column exists.
    pub async fn find_public_newsletter(
        username: String,
        slug: String,
        db_pool: &PgPool,
    ) -> Result<PublicNewsletter, sqlx::Error> {
        sqlx::query_as!(
            PublicNewsletter,
            r#"
              SELECT
                newsletter_issues.content,
                newsletter_issues.cover_image_url,
                newsletter_issues.description,
                newsletter_issues.published_at,
                newsletter_issues.slug,
                newsletter_issues.title,
                (
                  user_profiles.avatar_url,
                  user_profiles.banner_url,
                  user_profiles.description,
                  user_profiles.display_name,
                  users.username
                ) AS "user!: AssociatedUser"
              FROM newsletter_issues
              JOIN users ON newsletter_issues.user_id = users.user_id
              JOIN user_profiles ON newsletter_issues.user_id = user_profiles.user_id
              WHERE newsletter_issues.published_at IS NOT NULL
                AND users.username = $1
                AND newsletter_issues.slug = $2
              LIMIT 1
            "#,
            username,
            slug
        )
        .fetch_one(db_pool)
        .await
    }

    pub async fn get_public_newsletters(
        db_pool: &PgPool,
    ) -> Result<Vec<PublicNewsletterListItem>, sqlx::Error> {
        sqlx::query_as!(
            PublicNewsletterListItem,
            r#"
              SELECT
                newsletter_issues.cover_image_url,
                newsletter_issues.description,
                newsletter_issues.published_at,
                newsletter_issues.slug,
                newsletter_issues.title,
                (
                  user_profiles.avatar_url,
                  user_profiles.banner_url,
                  user_profiles.description,
                  user_profiles.display_name,
                  users.username
                ) AS "user!: AssociatedUser"
              FROM newsletter_issues
              JOIN users ON newsletter_issues.user_id = users.user_id
              JOIN user_profiles ON newsletter_issues.user_id = user_profiles.user_id
              WHERE newsletter_issues.published_at IS NOT NULL
              ORDER BY published_at DESC
              LIMIT 10
            "#,
        )
        .fetch_all(db_pool)
        .await
    }

    pub async fn get_public_newsletters_by_username(
        username: String,
        db_pool: &PgPool,
    ) -> Result<Vec<PublicNewsletterListItem>, sqlx::Error> {
        sqlx::query_as!(
            PublicNewsletterListItem,
            r#"
              SELECT
                newsletter_issues.cover_image_url,
                newsletter_issues.description,
                newsletter_issues.published_at,
                newsletter_issues.slug,
                newsletter_issues.title,
                (
                  user_profiles.avatar_url,
                  user_profiles.banner_url,
                  user_profiles.description,
                  user_profiles.display_name,
                  users.username
                ) AS "user!: AssociatedUser"
              FROM newsletter_issues
              JOIN users ON newsletter_issues.user_id = users.user_id
              JOIN user_profiles ON newsletter_issues.user_id = user_profiles.user_id
              WHERE newsletter_issues.published_at IS NOT NULL
                AND users.username = $1
              ORDER BY published_at DESC
              LIMIT 10
            "#,
            username
        )
        .fetch_all(db_pool)
        .await
    }

    pub fn validate_for_publish(self) -> Result<Self, String> {
        let content = Content::parse(self.content)?;
        let description = Description::parse(self.description)?;
        let title = Title::parse(self.title)?;

        Ok(Self {
            content: content.as_ref().to_string(),
            cover_image_url: self.cover_image_url,
            created_at: self.created_at,
            description: description.as_ref().to_string(),
            newsletter_issue_id: self.newsletter_issue_id,
            published_at: self.published_at,
            slug: self.slug,
            title: title.as_ref().to_string(),
            user_id: self.user_id,
        })
    }

    pub fn prepare_cover_image_url(
        newsletter_issue_id: &Uuid,
        s3_base_url: &str,
    ) -> Result<String, String> {
        let image_url = format!("{s3_base_url}/images/newsletter/cover/{newsletter_issue_id}.webp");
        let image_url = ImageUrl::parse(image_url)?.as_ref().to_string();

        Ok(image_url)
    }

    pub fn set_cover_image_url(self, s3_base_url: &str, is_empty: bool) -> Result<Self, String> {
        let cover_image_url = if is_empty {
            String::from("")
        } else {
            let timestamp: u64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            format!(
                "{}?v={timestamp}",
                Self::prepare_cover_image_url(&self.newsletter_issue_id, s3_base_url)?
            )
        };

        Ok(Self {
            content: self.content,
            cover_image_url,
            created_at: self.created_at,
            description: self.description,
            newsletter_issue_id: self.newsletter_issue_id,
            published_at: self.published_at,
            slug: self.slug,
            title: self.title,
            user_id: self.user_id,
        })
    }

    pub async fn process_image(
        self,
        file: String,
        s3_client: &S3Client,
        cloudinary_client: &CloudinaryClient,
    ) -> Result<Self, actix_web::Error> {
        Self::validate_and_upload_image(
            file,
            &self.newsletter_issue_id,
            s3_client,
            cloudinary_client,
        )
        .await?;

        Ok(self)
    }

    pub async fn validate_and_upload_image(
        image: String,
        newsletter_issue_id: &Uuid,
        s3_client: &S3Client,
        cloudinary_client: &CloudinaryClient,
    ) -> Result<(), actix_web::Error> {
        if is_empty_or_whitespace(&image) {
            return Ok(());
        }

        let uploaded_image = cloudinary_client
            .upload_newsletter_issue_cover_image(image, newsletter_issue_id)
            .await?;
        let content = cloudinary_client.get_image_as_bytes(uploaded_image).await?;
        s3_client
            .put_newsletter_issue_cover_image(newsletter_issue_id, content)
            .await
            .map_err(e500)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewsletterIssueAPI {
    pub content: String,
    pub cover_image_url: String,
    pub description: String,
    pub html_content: String,
    pub newsletter_issue_id: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
    pub title: String,
    pub user_id: Uuid,
}

impl From<NewsletterIssue> for NewsletterIssueAPI {
    fn from(newsletter_issue: NewsletterIssue) -> Self {
        let html_content = markdown::to_html(&newsletter_issue.content);

        Self {
            content: newsletter_issue.content,
            cover_image_url: newsletter_issue.cover_image_url,
            description: newsletter_issue.description,
            html_content,
            newsletter_issue_id: newsletter_issue.newsletter_issue_id,
            published_at: newsletter_issue.published_at,
            slug: newsletter_issue.slug,
            title: newsletter_issue.title,
            user_id: newsletter_issue.user_id,
        }
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
    pub cover_image_url: String,
    pub cover_image: String,
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

    pub async fn process_image(
        self,
        s3_client: &S3Client,
        cloudinary_client: &CloudinaryClient,
    ) -> Result<Self, actix_web::Error> {
        NewsletterIssue::validate_and_upload_image(
            self.cover_image.clone(),
            &self.newsletter_issue_id,
            s3_client,
            cloudinary_client,
        )
        .await?;

        Ok(self)
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
                    cover_image_url,
                    created_at,
                    description,
                    newsletter_issue_id,
                    published_at,
                    slug,
                    title,
                    user_id
                  )
                  VALUES ($1, $2, now(), $3, $4, NULL, $5, $6, $7)
                "#,
                self.content,
                self.cover_image_url,
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
    pub cover_image: String,
    pub description: String,
    pub s3_base_url: String,
    pub title: String,
}

impl TryFrom<NewNewsletterIssueData> for NewNewsletterIssue {
    type Error = String;

    fn try_from(data: NewNewsletterIssueData) -> Result<NewNewsletterIssue, String> {
        let cover_image = if is_empty_or_whitespace(&data.cover_image) {
            String::from("")
        } else {
            Base64ImageUrl::parse(data.cover_image)?
                .validate_size_limit(1024 * 1024 * 3)?
                .as_ref()
                .to_string()
        };
        let description = Description::parse_draft(data.description)?;
        let newsletter_issue_id = Uuid::new_v4();
        let title = Title::parse(data.title)?;
        let cover_image_url = if is_empty_or_whitespace(&cover_image) {
            String::from("")
        } else {
            NewsletterIssue::prepare_cover_image_url(&newsletter_issue_id, &data.s3_base_url)?
        };

        Ok(NewNewsletterIssue {
            content: data.content,
            cover_image_url,
            cover_image,
            description: description.as_ref().to_string(),
            newsletter_issue_id,
            slug: slug::slugify(title.as_ref()),
            title: title.as_ref().to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublicNewsletter {
    #[serde(serialize_with = "serialize_html_content")]
    pub content: String,
    pub cover_image_url: String,
    pub description: String,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
    pub title: String,
    pub user: AssociatedUser,
}

fn serialize_html_content<S: Serializer>(content: &str, serializer: S) -> Result<S::Ok, S::Error> {
    markdown::to_html(content).serialize(serializer)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublicNewsletterListItem {
    pub cover_image_url: String,
    pub description: String,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
    pub title: String,
    pub user: AssociatedUser,
}

#[cfg(test)]
mod tests {
    use crate::models::{
        NewNewsletterIssue, NewNewsletterIssueData, NewsletterIssue, NewsletterIssueAPI,
        NewsletterIssueEmail,
    };
    use chrono::Utc;
    use claims::{assert_err, assert_ok};
    use uuid::Uuid;

    #[test]
    fn valid_new_newsletter_issue_data_can_convert_into_newsletter_issue() {
        let test_newsletter_issue = NewNewsletterIssueData {
            content: String::from("## Newsletter content"),
            description: String::from("Newsletter description"),
            title: String::from("Ursula Le Guin"),
            cover_image: String::from(""),
            s3_base_url: String::from(""),
        };

        assert_ok!(NewNewsletterIssue::try_from(test_newsletter_issue));
    }

    #[test]
    fn invalid_new_newsletter_issue_data_cannot_convert_into_newsletter_issue() {
        let test_newsletter_issue = NewNewsletterIssueData {
            content: String::from("## Newsletter content"),
            description: String::from("Newsletter description"),
            title: String::from("Ursula>Le Guin"),
            cover_image: String::from(""),
            s3_base_url: String::from(""),
        };

        assert_err!(NewNewsletterIssue::try_from(test_newsletter_issue));
    }

    #[test]
    fn newsletter_issue_api_correctly_parses_and_sanitizes_content() {
        let new_newsletter_issue: NewNewsletterIssue = NewNewsletterIssueData {
            content: String::from("## Newsletter content"),
            description: String::from("Newsletter description"),
            title: String::from("Ursula Le Guin"),
            cover_image: String::from(""),
            s3_base_url: String::from(""),
        }
        .try_into()
        .unwrap();
        let newsletter_issue_api = NewsletterIssueAPI::from(NewsletterIssue {
            content: new_newsletter_issue.content,
            cover_image_url: String::from(""),
            created_at: Utc::now(),
            description: new_newsletter_issue.description,
            newsletter_issue_id: new_newsletter_issue.newsletter_issue_id,
            published_at: Some(Utc::now()),
            slug: new_newsletter_issue.slug,
            title: new_newsletter_issue.title,
            user_id: Uuid::new_v4(),
        });

        assert_eq!(newsletter_issue_api.slug, "ursula-le-guin");
        assert_eq!(
            newsletter_issue_api.html_content,
            "<h2>Newsletter content</h2>"
        );
    }

    #[test]
    fn newsletter_issue_email_correctly_parses_and_sanitizes_content() {
        let new_newsletter_issue: NewNewsletterIssue = NewNewsletterIssueData {
            content: String::from("## Newsletter content"),
            description: String::from("Newsletter description"),
            title: String::from("Ursula Le Guin"),
            cover_image: String::from(""),
            s3_base_url: String::from(""),
        }
        .try_into()
        .unwrap();
        let newsletter_issue_email = NewsletterIssueEmail::from(NewsletterIssue {
            content: new_newsletter_issue.content,
            cover_image_url: String::from(""),
            created_at: Utc::now(),
            description: new_newsletter_issue.description,
            newsletter_issue_id: new_newsletter_issue.newsletter_issue_id,
            published_at: Some(Utc::now()),
            slug: new_newsletter_issue.slug,
            title: new_newsletter_issue.title,
            user_id: Uuid::new_v4(),
        });

        assert_eq!(newsletter_issue_email.slug, "ursula-le-guin");
        assert_eq!(
            newsletter_issue_email.html_content,
            "<h2>Newsletter content</h2>"
        );
        assert_eq!(newsletter_issue_email.text_content, "Newsletter content");
    }
}
