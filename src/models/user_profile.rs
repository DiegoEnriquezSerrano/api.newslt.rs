use crate::domain::ImageUrl;
use crate::domain::user_profile::{Description, DisplayName};
use crate::utils::{e400, e500};
use anyhow::Context;
use serde::{Deserialize, Serialize, Serializer};
use sqlx::{Executor, PgPool, Postgres, Transaction, Type};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserProfile {
    pub bio: String,
    pub description: String,
    pub display_name: String,
    pub user_id: Uuid,
}

impl UserProfile {
    pub fn initialize(user_id: &Uuid) -> Self {
        Self {
            bio: "".to_string(),
            description: "".to_string(),
            display_name: "".to_string(),
            user_id: *user_id,
        }
    }

    pub fn validate(self) -> Result<UserProfile, String> {
        let description = Description::parse(self.description)?.as_ref().to_string();
        let display_name = DisplayName::parse(self.display_name)?.as_ref().to_string();

        Ok(Self {
            bio: self.bio,
            description,
            display_name,
            user_id: self.user_id,
        })
    }

    pub async fn find_user_profile_api_by_user_id(
        user_id: Uuid,
        pool: &PgPool,
    ) -> Result<UserProfileAPI, sqlx::Error> {
        sqlx::query_as!(
            UserProfileAPI,
            r#"
              SELECT
                avatar_url,
                banner_url,
                bio,
                bio AS "bio_html!: String",
                description,
                display_name,
                username,
                (
                  SELECT COUNT(*)
                  FROM newsletter_issues
                  WHERE published_at IS NOT NULL
                    AND users.user_id = newsletter_issues.user_id
                ) as "total_issues"
              FROM users
              JOIN user_profiles ON users.user_id = user_profiles.user_id
              WHERE users.user_id = $1
            "#,
            user_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_public_profiles(
        pool: &PgPool,
    ) -> Result<Vec<PublicProfileListItem>, sqlx::Error> {
        sqlx::query_as!(
            PublicProfileListItem,
            r#"
              SELECT
                description,
                display_name,
                username,
                (
                  SELECT COUNT(*)
                  FROM newsletter_issues
                  WHERE published_at IS NOT NULL
                    AND users.user_id = newsletter_issues.user_id
                ) AS "total_issues!: i64"
              FROM users
              JOIN user_profiles ON users.user_id = user_profiles.user_id
              LIMIT 10
            "#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_public_profile(
        username: String,
        pool: &PgPool,
    ) -> Result<PublicProfile, sqlx::Error> {
        sqlx::query_as!(
            PublicProfile,
            r#"
              SELECT
                bio,
                description,
                display_name,
                username,
                (
                  SELECT COUNT(*)
                  FROM newsletter_issues
                  WHERE published_at IS NOT NULL
                    AND users.user_id = newsletter_issues.user_id
                ) AS "total_issues!: i64"
              FROM users
              JOIN user_profiles ON users.user_id = user_profiles.user_id
              WHERE users.username = $1
            "#,
            username
        )
        .fetch_one(pool)
        .await
    }

    pub async fn set_banner(
        user_id: &Uuid,
        s3_base_url: &str,
        pool: &PgPool,
    ) -> Result<(), actix_web::Error> {
        let banner_url = format!("{s3_base_url}/images/user/banner/{user_id}.webp");
        let banner_url = ImageUrl::parse(banner_url)
            .map_err(e400)?
            .as_ref()
            .to_string();

        Self::update_banner(user_id, &banner_url, pool)
            .await
            .context("Failed to update user banner.")
            .map_err(e500)?;

        Ok(())
    }

    pub async fn insert(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        transaction
            .execute(sqlx::query!(
                r#"
                  INSERT INTO user_profiles (
                    bio,
                    description,
                    display_name,
                    user_profile_id,
                    user_id
                  )
                  VALUES ($1, $2, $3, $4, $5)
                "#,
                self.bio,
                self.description,
                self.display_name,
                Uuid::new_v4(),
                self.user_id
            ))
            .await?;

        Ok(())
    }

    pub async fn update(&self, db_pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
              UPDATE user_profiles
              SET bio = $1,
                  description = $2,
                  display_name = $3
              WHERE user_id = $4
            "#,
            self.bio,
            self.description,
            self.display_name,
            self.user_id
        )
        .execute(db_pool)
        .await?;

        Ok(())
    }

    async fn update_banner(
        user_id: &Uuid,
        banner_url: &String,
        db_pool: &PgPool,
    ) -> Result<(), sqlx::Error> {
        let timestamp: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let banner_url = format!("{banner_url}?v={timestamp}");

        sqlx::query!(
            r#"
              UPDATE user_profiles
              SET banner_url = $1
              WHERE user_id = $2
            "#,
            banner_url,
            user_id
        )
        .execute(db_pool)
        .await?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserProfileAPI {
    pub avatar_url: String,
    pub banner_url: String,
    pub bio: String,
    #[serde(serialize_with = "serialize_bio")]
    pub bio_html: String,
    pub description: String,
    pub display_name: String,
    pub username: String,
    pub total_issues: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct AssociatedUser {
    pub avatar_url: String,
    pub banner_url: String,
    pub description: String,
    pub display_name: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublicProfile {
    #[serde(serialize_with = "serialize_bio")]
    pub bio: String,
    pub description: String,
    pub display_name: String,
    pub username: String,
    pub total_issues: i64,
}

fn serialize_bio<S: Serializer>(bio: &str, serializer: S) -> Result<S::Ok, S::Error> {
    markdown::to_html(bio).serialize(serializer)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublicProfileListItem {
    pub description: String,
    pub display_name: String,
    pub username: String,
    pub total_issues: i64,
}

#[cfg(test)]
mod tests {
    use crate::models::UserProfile;
    use claims::{assert_err, assert_ok};
    use uuid::Uuid;

    #[test]
    fn valid_user_profile_data_can_convert_into_user() {
        let test_user = UserProfile {
            bio: "".to_string(),
            description: "some valid text".to_string(),
            display_name: "".to_string(),
            user_id: Uuid::new_v4(),
        };
        assert_ok!(test_user.validate());
    }

    #[test]
    fn invalid_user_profile_data_cannot_convert_into_user() {
        let test_user = UserProfile {
            bio: "".to_string(),
            description: "some valid text".to_string(),
            display_name: "Ur/<>sula-Le-Guin".to_string(),
            user_id: Uuid::new_v4(),
        };
        assert_err!(test_user.validate());

        let test_user = UserProfile {
            bio: "".to_string(),
            description: "Ur/<>sula-Le-Guin".to_string(),
            display_name: "".to_string(),
            user_id: Uuid::new_v4(),
        };
        assert_err!(test_user.validate());
    }
}
