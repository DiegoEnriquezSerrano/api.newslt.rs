use crate::helpers::spawn_app;
use newsletter_api::models::{NewsletterIssueAPI, PublicNewsletter};

#[tokio::test]
async fn nonexistent_path_params_return_not_found() {
    let app = spawn_app().await;

    let response = app
        .get_public_newsletter(&"fakeuser".to_string(), &"fakeslug".to_string())
        .await;
    assert_eq!(404, response.status().as_u16());
}

#[tokio::test]
async fn existing_path_params_return_newsletter() {
    // Act 1 - Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    // Act 2 - Publish newsletter issue
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter body as HTML",
      "cover_image": "",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    app.put_admin_publish_newsletter(
        &newsletter_issue_id,
        &serde_json::json!({
          "idempotency_key": uuid::Uuid::new_v4().to_string()
        }),
    )
    .await;
    app.post_logout().await;

    // Act 3 - Retrieve endpoint for existing issue as unauthenticated user
    let response = app
        .get_public_newsletter(&app.test_user.username, &"newsletter-title".to_string())
        .await;
    assert_eq!(200, response.status().as_u16());

    let response_body: PublicNewsletter = response.json().await.unwrap();
    assert_eq!(&response_body.title, "Newsletter title");
    assert_eq!(&response_body.description, "Newsletter description");
    assert_eq!(&response_body.content, "<h2>Newsletter body as HTML</h2>",);
}
