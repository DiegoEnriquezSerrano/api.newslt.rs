use crate::authentication::UserId;
use crate::clients::cloudinary_client::CloudinaryClient;
use crate::clients::s3_client::S3Client;
use crate::models::{
    NewNewsletterIssue, NewNewsletterIssueData, NewsletterIssue, NewsletterIssueAPI,
};
use crate::utils::{ResponseMessage, e400, e500};
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get, post, web};
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;

#[get("/newsletters")]
#[tracing::instrument(
    name = "Retrieving user's newsletter issues",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn get(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let newsletter_issues = NewsletterIssue::get_published_by_user_id(*user_id, &pool)
        .await
        .context("Failed to query newsletter issues.")
        .map_err(e500)?;
    let mut newsletter_issues_api_vec: Vec<NewsletterIssueAPI> = vec![];

    for newsletter_issue in newsletter_issues {
        newsletter_issues_api_vec.push(NewsletterIssueAPI::from(newsletter_issue));
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(newsletter_issues_api_vec))
}

#[derive(Deserialize)]
pub struct CreateNewsletterParams {
    content: String,
    cover_image: String,
    description: String,
    title: String,
}

#[post("/newsletters")]
#[tracing::instrument(
    name = "Creating a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn post(
    params: web::Json<CreateNewsletterParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    s3_client: web::Data<S3Client>,
    cloudinary_client: web::Data<CloudinaryClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let new_newsletter_issue: NewNewsletterIssue = NewNewsletterIssueData {
        content: params.0.content,
        cover_image: params.0.cover_image,
        description: params.0.description,
        s3_base_url: s3_client.endpoint.clone(),
        title: params.0.title,
    }
    .try_into()
    .map_err(e400)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to begin database transaction.")
        .map_err(e500)?;
    new_newsletter_issue
        .validate(&user_id, &mut transaction)
        .await
        .map_err(e400)?
        .process_image(s3_client.get_ref(), cloudinary_client.get_ref())
        .await?
        .insert_newsletter_issue(&user_id, &mut transaction)
        .await
        .context("Failed to store newsletter issue details.")
        .map_err(e500)?;
    transaction
        .commit()
        .await
        .context("Failed to commit create newsletter transaction.")
        .map_err(e500)?;

    Ok(HttpResponse::Created()
        .content_type(ContentType::json())
        .json(ResponseMessage::from(
            "The newsletter issue has been created.",
        )))
}
