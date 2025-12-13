use crate::helpers::spawn_app;
use newsletter_api::models::NewsletterIssueAPI;

#[tokio::test]
async fn unauthenticated_users_cannot_list_newsletter_drafts() {
    let app = spawn_app().await;

    let response = app.get_admin_unpublished_newsletter_issues().await;

    assert_eq!(401, response.status().as_u16());
}

#[tokio::test]
async fn authenticated_users_can_list_newsletter_drafts() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let response = app.get_admin_unpublished_newsletter_issues().await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn drafts_are_listed_by_created_at() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
      "idempotency_key": uuid::Uuid::new_v4().to_string()
    }))
    .await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title 2",
      "description": "Newsletter description 2",
      "content": "## Newsletter content 2",
      "idempotency_key": uuid::Uuid::new_v4().to_string()
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();

    assert_eq!("Newsletter title 2", response_body[0].title);
    assert_eq!("Newsletter title", response_body[1].title);
}
