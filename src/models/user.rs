use crate::domain::user::{Email, Username};
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{Executor, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
}

impl TryFrom<PgRow> for User {
    type Error = sqlx::Error;

    fn try_from(row: PgRow) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: row.try_get("user_id")?,
            username: row.try_get("username")?,
        })
    }
}

impl User {
    pub async fn find_by_username(
        username: &str,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<User, sqlx::Error> {
        transaction
            .fetch_one(sqlx::query_as!(
                User,
                r#"
                  SELECT user_id, username
                  FROM users
                  WHERE username = $1
                "#,
                username
            ))
            .await?
            .try_into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub user_id: Uuid,
    pub username: String,
    is_superuser: bool,
}

impl TryFrom<NewUserData> for NewUser {
    type Error = String;

    fn try_from(data: NewUserData) -> Result<NewUser, Self::Error> {
        let user = prepare_new_user(data)?;

        Ok(NewUser {
            email: user.email,
            is_superuser: user.is_superuser,
            password_hash: user.password_hash,
            user_id: user.user_id,
            username: user.username,
        })
    }
}

impl NewUser {
    pub async fn store(
        self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Self, anyhow::Error> {
        transaction
            .execute(sqlx::query!(
                r#"
                  INSERT INTO users (
                    email,
                    is_superuser,
                    password_hash,
                    user_id,
                    username
                  )
                  VALUES ($1, $2, $3, $4, $5)
                "#,
                self.email,
                self.is_superuser,
                self.password_hash,
                self.user_id,
                self.username
            ))
            .await
            .context("Failed to store new user.")?;

        Ok(self)
    }

    pub fn make_superuser(mut self) -> Self {
        self.is_superuser = true;
        self
    }
}

#[derive(Deserialize, Debug)]
pub struct NewUserData {
    pub email: String,
    pub username: String,
    pub password: Secret<String>,
}

fn prepare_new_user(data: NewUserData) -> Result<NewUser, String> {
    let user_id = Uuid::new_v4();
    let email = Email::parse(data.email)?.as_ref().to_string();
    let username = Username::parse(data.username)?.as_ref().to_string();
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(data.password.expose_secret().as_bytes(), &salt)
    .unwrap()
    .to_string();

    Ok(NewUser {
        email,
        is_superuser: false,
        password_hash,
        user_id,
        username,
    })
}

#[cfg(test)]
mod tests {
    use crate::models::{NewUser, NewUserData};
    use claims::{assert_err, assert_ok};
    use secrecy::Secret;

    #[test]
    fn valid_new_user_data_can_convert_into_user() {
        let test_user = NewUserData {
            email: "myguy@example.com".to_string(),
            password: Secret::new("secretpassword".to_string()),
            username: "Ursula-Le-Guin".to_string(),
        };

        assert_ok!(NewUser::try_from(test_user));
    }

    #[test]
    fn invalid_new_user_data_cannot_convert_into_user() {
        let test_user = NewUserData {
            email: "myguy@example.com".to_string(),
            password: Secret::new("secretpassword".to_string()),
            username: "Ursula Le Guin".to_string(),
        };

        assert_err!(NewUser::try_from(test_user));
    }
}
