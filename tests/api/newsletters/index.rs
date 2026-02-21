use crate::helpers::spawn_app;
use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use newsletter_api::models::{
    NewUser, NewUserData, NewsletterIssueAPI, PublicNewsletterListItem, UserProfile,
};
use secrecy::SecretString;

#[tokio::test]
async fn unauthenticated_user_can_get_published_timeline_newsletter_issues() {
    let app = spawn_app().await;

    let response = app.get_public_newsletters().await;
    assert_eq!(200, response.status().as_u16());

    let response_body: Vec<PublicNewsletterListItem> = response.json().await.unwrap();
    assert_eq!(response_body.is_empty(), true);
}

#[tokio::test]
async fn published_timeline_newsletters_are_listed_by_published_at() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    // Act 1 - Create newsletter records
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title 0",
      "description": "Newsletter description 0",
      "content": "## Newsletter content 0",
      "cover_image": "",
    }))
    .await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title 1",
      "description": "Newsletter description 1",
      "content": "## Newsletter content 1",
      "cover_image": "",
    }))
    .await;

    // Act 2 - Publish first newsletter record
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();

    assert_eq!("Newsletter title 1".to_string(), response_body[0].title);

    app.put_admin_publish_newsletter(
        &response_body[0].newsletter_issue_id,
        &serde_json::json!({
          "idempotency_key": uuid::Uuid::new_v4().to_string()
        }),
    )
    .await;

    // Act 3 - Publish subsequent newsletter record
    assert_eq!("Newsletter title 0".to_string(), response_body[1].title);

    app.put_admin_publish_newsletter(
        &response_body[1].newsletter_issue_id,
        &serde_json::json!({
          "idempotency_key": uuid::Uuid::new_v4().to_string()
        }),
    )
    .await;

    // Act 4 - Request endpoint as unauthenticated user
    let response = app.post_logout().await;
    assert_eq!(200, response.status().as_u16());

    let response = app.get_public_newsletters().await;
    let response_body: Vec<PublicNewsletterListItem> = response.json().await.unwrap();

    assert_eq!("Newsletter title 0", response_body[0].title);
    assert_eq!("Newsletter title 1", response_body[1].title);
}

#[tokio::test]
async fn all_users_published_timeline_newsletters_are_listed() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    // Act 1 - Create first user's newsletter issue
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title 0",
      "description": "Newsletter description 0",
      "content": "## Newsletter content 0",
      "cover_image": "",
    }))
    .await;

    // Act 2 - Publish first user's newsletter issue
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response
        .json()
        .await
        .expect("Failed to serialize newsletter issue api response.");
    let newsletter_issue_0 = &response_body[0];
    let newsletter_issue_id_0 = &newsletter_issue_0.newsletter_issue_id;
    app.put_admin_publish_newsletter(
        newsletter_issue_id_0,
        &serde_json::json!({
          "idempotency_key": uuid::Uuid::new_v4().to_string()
        }),
    )
    .await;
    app.post_logout().await;

    // Act 3 - Create second user
    let mut transaction = app
        .db_pool
        .begin()
        .await
        .expect("Failed to begin transaction.");
    let second_user = NewUser::try_from(NewUserData {
        username: uuid::Uuid::new_v4().to_string(),
        email: SafeEmail().fake(),
        password: SecretString::from("testpassword"),
    })
    .expect("Failed to initialize new user.")
    .store(&mut transaction)
    .await
    .expect("Failed to persist user.");
    UserProfile::initialize(&second_user.user_id)
        .insert(&mut transaction)
        .await
        .expect("Failed to persist user profile.");
    transaction
        .commit()
        .await
        .expect("Failed to commit transaction.");

    // Act 4 - Create second user's newsletter
    app.post_login(
        &serde_json::json!({"username": second_user.username, "password": "testpassword"}),
    )
    .await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title 1",
      "description": "Newsletter description 1",
      "content": "## Newsletter content 1",
      "cover_image": "",
    }))
    .await;

    // Act 5 - Publish second user's newsletter
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response
        .json()
        .await
        .expect("Failed to serialize unpublished newsletters response body.");
    let newsletter_issue_1 = &response_body[0];
    let newsletter_issue_id_1 = &newsletter_issue_1.newsletter_issue_id;
    app.put_admin_publish_newsletter(
        newsletter_issue_id_1,
        &serde_json::json!({
          "idempotency_key": uuid::Uuid::new_v4().to_string()
        }),
    )
    .await;
    app.post_logout().await;

    // Act 6 - Get public newsletter issues as unauthenticated user
    let response = app.get_public_newsletters().await;
    let response_body: Vec<PublicNewsletterListItem> = response
        .json()
        .await
        .expect("Failed to serialize public newsletter issues.");

    assert_eq!(newsletter_issue_1.title, response_body[0].title);
    assert_eq!(newsletter_issue_0.title, response_body[1].title);
}
