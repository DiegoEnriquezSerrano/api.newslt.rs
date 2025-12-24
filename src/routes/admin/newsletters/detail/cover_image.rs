use crate::authentication::UserId;
use crate::clients::cloudinary_client::CloudinaryClient;
use crate::clients::s3_client::S3Client;
use crate::domain::Base64ImageUrl;
use crate::models::NewsletterIssue;
use crate::utils::{e400, e404, e500};
use actix_web::{HttpResponse, put, web};
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
struct NewsletterIssueCoverImageParams {
    pub image: String,
}

#[put("/newsletter/{newsletter_issue_id}/cover_image")]
#[tracing::instrument(
  name = "Updating a newsletter issue's cover image",
  skip_all,
  fields(user_id=%*user_id)
)]
pub async fn put(
    cloudinary_client: web::Data<CloudinaryClient>,
    params: web::Json<NewsletterIssueCoverImageParams>,
    path: web::Path<(Uuid,)>,
    pool: web::Data<PgPool>,
    s3_client: web::Data<S3Client>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let newsletter_issue_id = path.into_inner().0;
    let user_id = user_id.into_inner();
    let image = Base64ImageUrl::parse(params.0.image)
        .map_err(e400)?
        .validate_size_limit(1024 * 1024 * 3)
        .map_err(e400)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to begin database transaction.")
        .map_err(e500)?;
    NewsletterIssue::find_by_user_id_and_newsletter_issue_id_txn(
        *user_id,
        &newsletter_issue_id,
        &mut transaction,
    )
    .await
    .map_err(e404)?
    .process_image(image.as_ref().to_string(), &s3_client, &cloudinary_client)
    .await?
    .set_cover_image_url(&s3_client.endpoint, image.as_ref().trim().is_empty())
    .map_err(e400)?
    .update(&mut transaction)
    .await
    .context("Failed to update newsletter issue cover image.")
    .map_err(e500)?;
    transaction
        .commit()
        .await
        .context("Failed to commit transaction.")
        .map_err(e500)?;

    Ok(HttpResponse::Ok().finish())
}
