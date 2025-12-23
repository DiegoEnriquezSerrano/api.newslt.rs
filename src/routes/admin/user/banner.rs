use crate::authentication::UserId;
use crate::clients::cloudinary_client::CloudinaryClient;
use crate::clients::s3_client::S3Client;
use crate::domain::Base64ImageUrl;
use crate::models::UserProfile;
use crate::utils::{e400, e500};
use actix_web::{HttpResponse, put, web};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
struct UpdateBannerParams {
    pub image: String,
}

#[put("/user/banner")]
#[tracing::instrument(
  name = "Updating user profile banner",
  skip_all,
  fields(user_id=%*user_id)
)]
pub async fn put(
    cloudinary_client: web::Data<CloudinaryClient>,
    params: web::Json<UpdateBannerParams>,
    pool: web::Data<PgPool>,
    s3_client: web::Data<S3Client>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = *user_id.into_inner();
    let image = Base64ImageUrl::parse(params.0.image)
        .map_err(e400)?
        .validate_size_limit(1024 * 1024 * 3)
        .map_err(e400)?;
    let uploaded_image = cloudinary_client
        .upload_banner(image.as_ref().to_string(), &user_id)
        .await?;
    let content = cloudinary_client.get_image_as_bytes(uploaded_image).await?;
    s3_client
        .put_user_profile_banner(&user_id, content)
        .await
        .map_err(e500)?;
    UserProfile::set_banner(&user_id, &s3_client.endpoint, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}
