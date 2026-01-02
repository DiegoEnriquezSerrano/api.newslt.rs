use crate::challenge::{Base64Challenger, CaptchaResponse};
use crate::startup::CaptchaSecret;
use crate::utils::e500;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, web};

#[get("/captcha")]
#[tracing::instrument(name = "Generating a new captcha challenge", skip(captcha_secret))]
pub async fn get(
    captcha_secret: web::Data<CaptchaSecret>,
) -> Result<HttpResponse, actix_web::Error> {
    let challenger = Base64Challenger::new(captcha_secret.0.clone()).map_err(e500)?;
    let challenge = challenger.encrypt().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(CaptchaResponse {
            challenge_image: format!("data:image/png;base64,{}", challenger.base64_image),
            challenge,
        }))
}
