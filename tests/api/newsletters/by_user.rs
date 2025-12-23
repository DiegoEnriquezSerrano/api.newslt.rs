use crate::helpers::spawn_app;
use newsletter_api::models::{NewsletterIssueAPI, PublicNewsletterListItem};

#[tokio::test]
async fn returns_ok_for_non_existing_username() {
    let app = spawn_app().await;

    let response = app
        .get_public_newsletters_by_user(&"fakeuser".to_string())
        .await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn returns_ok_for_existing_username() {
    let app = spawn_app().await;

    let response = app
        .get_public_newsletters_by_user(&app.test_user.username)
        .await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn retuns_published_public_newsletters_by_user() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    // Act 1 - Create a newsletter
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
      "cover_image": "",
    }))
    .await;

    // Act 2 - Publish newsletter
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();

    app.put_admin_publish_newsletter(
        &response_body[0].newsletter_issue_id,
        &serde_json::json!({
          "idempotency_key": uuid::Uuid::new_v4().to_string()
        }),
    )
    .await;
    app.post_logout().await;

    // Act 3 - Request endpoint as an unauthenticated user
    let response = app
        .get_public_newsletters_by_user(&app.test_user.username)
        .await;
    let response_body: Vec<PublicNewsletterListItem> = response.json().await.unwrap();
    assert_eq!("Newsletter title", response_body[0].title);
}
