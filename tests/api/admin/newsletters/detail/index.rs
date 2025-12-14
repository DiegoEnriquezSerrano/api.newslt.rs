use crate::helpers::spawn_app;
use newsletter_api::models::{NewUser, NewUserData, NewsletterIssueAPI};
use newsletter_api::utils::ResponseErrorMessage;
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

#[tokio::test]
async fn authenticated_user_can_update_a_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    let response = app
        .put_admin_update_newsletter(
            &newsletter_issue_id,
            &serde_json::json!({
              "title": "Newsletter title - updated",
              "description": "Newsletter description - updated",
              "content": "## Newsletter content - updated",
            }),
        )
        .await;
    assert_eq!(200, response.status().as_u16());

    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    let response_body: NewsletterIssueAPI = response.json().await.unwrap();
    assert_eq!(
        String::from("Newsletter title - updated"),
        response_body.title
    );
}

#[tokio::test]
async fn unauthenticated_user_cannot_update_a_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    app.post_logout().await;

    let response = app
        .put_admin_update_newsletter(
            &newsletter_issue_id,
            &serde_json::json!({
              "title": "Newsletter title - updated",
              "description": "Newsletter description - updated",
              "content": "## Newsletter content",
            }),
        )
        .await;
    assert_eq!(401, response.status().as_u16());

    app.test_user.login(&app).await;

    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    let response_body: NewsletterIssueAPI = response.json().await.unwrap();
    assert_eq!(String::from("Newsletter title"), response_body.title);
}

#[tokio::test]
async fn newsletter_update_returns_400_for_missing_fields() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;
    let test_cases = vec![
        (
            serde_json::json!({
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "missing title",
        ),
        (
            serde_json::json!({
              "title": "Newsletter!",
              "description": "Newsletter description",
            }),
            "missing content",
        ),
        (
            serde_json::json!({
              "title": "Newsletter!",
              "content": "## Newsletter content",
            }),
            "missing description",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app
            .put_admin_update_newsletter(&newsletter_issue_id, &invalid_body)
            .await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn draft_newsletter_update_returns_400_for_invalid_fields() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;
    let test_cases = vec![
        (
            serde_json::json!({
              "title": " ",
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "A title is required.",
            "empty title",
        ),
        (
            serde_json::json!({
              "title": "a".repeat(71),
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "Title exceeds character limit.",
            "excessively large title",
        ),
        (
            serde_json::json!({
              "title": "a</>a",
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "Title includes illegal characters.",
            "title with illegal characters",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "a".repeat(201),
              "content": "## Newsletter content",
            }),
            "Description exceeds character limit.",
            "excessively large description",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "<p>Newsletter description</p>",
              "content": "## Newsletter content",
            }),
            "Description includes illegal characters.",
            "description with illegal characters",
        ),
    ];

    for (invalid_body, error_message, test_case) in test_cases {
        let response = app
            .put_admin_update_newsletter(&newsletter_issue_id, &invalid_body)
            .await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the condition was {test_case}.",
        );
        assert_eq!(
            error_message,
            response.json::<ResponseErrorMessage>().await.unwrap().error,
            "The API did not respond with '{error_message}' when the condition was {test_case}.",
        );
    }
}

#[tokio::test]
async fn newsletter_update_returns_400_for_invalid_fields() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    let response = app
        .put_admin_publish_newsletter(
            &newsletter_issue_id,
            &serde_json::json!({
              "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
        )
        .await;
    assert_eq!(200, response.status().as_u16());

    let test_cases = vec![
        (
            serde_json::json!({
              "title": " ",
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "A title is required.",
            "empty title",
        ),
        (
            serde_json::json!({
              "title": "a".repeat(71),
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "Title exceeds character limit.",
            "excessively large title",
        ),
        (
            serde_json::json!({
              "title": "a</>a",
              "description": "Newsletter description",
              "content": "## Newsletter content",
            }),
            "Title includes illegal characters.",
            "title with illegal characters",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "a".repeat(201),
              "content": "## Newsletter content",
            }),
            "Description exceeds character limit.",
            "excessively large description",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "<p>Newsletter description</p>",
              "content": "## Newsletter content",
            }),
            "Description includes illegal characters.",
            "description with illegal characters",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "Newsletter description",
              "content": " ",
            }),
            "Content body is required.",
            "empty content",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": " ",
              "content": "## Newsletter content",
            }),
            "A description is required.",
            "empty description",
        ),
    ];

    for (invalid_body, error_message, test_case) in test_cases {
        let response = app
            .put_admin_update_newsletter(&newsletter_issue_id, &invalid_body)
            .await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the condition was {test_case}.",
        );
        assert_eq!(
            error_message,
            response.json::<ResponseErrorMessage>().await.unwrap().error,
            "The API did not respond with '{error_message}' when the condition was {test_case}.",
        );
    }
}

#[tokio::test]
async fn authenticated_user_cannot_update_anothers_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
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

    let response = app
        .put_admin_update_newsletter(
            &newsletter_issue_id,
            &serde_json::json!({
              "title": "Newsletter title - updated",
              "description": "Newsletter description - updated",
              "content": "## Newsletter body as HTML - updated",
            }),
        )
        .await;
    assert_eq!(404, response.status().as_u16());
}
