use crate::authentication::{AuthError, Credentials, UserId, validate_credentials};
use crate::routes::admin::dashboard::get_username;
use crate::utils::{e400, e500};
use actix_web::{HttpResponse, put, web};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize, Debug)]
pub struct ChangePasswordParams {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[put("/password")]
pub async fn put(
    params: web::Json<ChangePasswordParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    validate_password(&params.new_password, &params.new_password_check).map_err(e400)?;
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await.map_err(e500)?;
    let credentials = Credentials {
        username,
        password: params.0.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Err(e400(e)),
            AuthError::ValidationError(_) => Err(e400(e)),
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(*user_id, params.0.new_password, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().finish())
}

fn validate_password(
    password: &Secret<String>,
    password_check: &Secret<String>,
) -> Result<(), String> {
    if password.expose_secret() != password_check.expose_secret() {
        Err(ValidationFailure::Mismatch.into())
    } else if password.expose_secret().len() <= 12 {
        Err(ValidationFailure::TooShort.into())
    } else if password.expose_secret().len() >= 129 {
        Err(ValidationFailure::TooLong.into())
    } else {
        Ok(())
    }
}

enum ValidationFailure {
    Mismatch,
    TooShort,
    TooLong,
}

impl From<ValidationFailure> for String {
    fn from(value: ValidationFailure) -> Self {
        match value {
            ValidationFailure::Mismatch => {
                "You entered two different new passwords - the field values must match.".to_string()
            }
            ValidationFailure::TooShort => {
                "New password must be longer than 12 characters.".to_string()
            }
            ValidationFailure::TooLong => {
                "New password must be shorter than 129 characters.".to_string()
            }
        }
    }
}
