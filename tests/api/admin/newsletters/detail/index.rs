use crate::helpers::spawn_app;
use newsletter_api::models::{NewUser, NewUserData, NewsletterIssueAPI};
use secrecy::Secret;

#[tokio::test]
async fn unauthenticated_user_cannot_fetch_a_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content"
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    app.post_logout().await;

    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    assert_eq!(401, response.status().as_u16());
}

#[tokio::test]
async fn authenticated_user_can_fetch_a_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content"
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    assert_eq!(200, response.status().as_u16());

    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    assert_eq!(200, response.status().as_u16());

    let response_body: NewsletterIssueAPI = response.json().await.unwrap();
    assert_eq!(newsletter_issue_id, response_body.newsletter_issue_id);
}

#[tokio::test]
async fn authenticated_user_cannot_view_anothers_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content"
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    app.post_logout().await;

    let second_user: NewUser = NewUserData {
        username: uuid::Uuid::new_v4().to_string(),
        email: String::from("seconduser@example.org"),
        password: Secret::new(String::from("testpassword")),
    }
    .try_into()
    .unwrap();

    let mut transaction = app.db_pool.begin().await.unwrap();
    let second_user = second_user.store(&mut transaction).await.unwrap();
    let _ = transaction.commit().await;

    app.post_login(
        &serde_json::json!({"username": second_user.username, "password": "testpassword"}),
    )
    .await;

    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    assert_eq!(404, response.status().as_u16());
}
