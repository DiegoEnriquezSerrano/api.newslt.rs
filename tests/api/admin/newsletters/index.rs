use crate::helpers::spawn_app;
use newsletter_api::utils::{ResponseErrorMessage, ResponseMessage};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn unauthenticated_users_cannot_list_newsletters() {
    let app = spawn_app().await;

    let response = app.get_admin_newsletter_issues().await;

    assert_eq!(401, response.status().as_u16());
}

#[tokio::test]
async fn authenticated_users_can_list_newsletters() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let response = app.get_admin_newsletter_issues().await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn authenticated_user_can_create_a_newsletter() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let response = app
        .post_admin_create_newsletter(&serde_json::json!({
          "title": "Newsletter title",
          "description": "Newsletter description",
          "content": "<p>Newsletter body as HTML</p>",
        }))
        .await;
    assert_eq!(201, response.status().as_u16());
    assert_eq!(
        String::from("The newsletter issue has been created."),
        response.json::<ResponseMessage>().await.unwrap().message
    );
}

#[tokio::test]
async fn unauthenticated_user_cannot_create_a_newsletter() {
    let app = spawn_app().await;
    let response = app
        .post_admin_create_newsletter(&serde_json::json!({
          "title": "Newsletter title",
          "description": "Newsletter description",
          "content": "<p>Newsletter body as HTML</p>",
        }))
        .await;

    assert_eq!(401, response.status().as_u16());
}

#[tokio::test]
async fn create_newsletter_returns_400_for_missing_fields() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let test_cases = vec![
        (
            serde_json::json!({
              "description": "Newsletter description",
              "content": "<p>Newsletter body as HTML</p>",
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
              "content": "<p>Newsletter body as HTML</p>",
            }),
            "missing description",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_admin_create_newsletter(&invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn create_newsletter_returns_400_for_invalid_fields() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let test_cases = vec![
        (
            serde_json::json!({
              "title": " ",
              "description": "Newsletter description",
              "content": "## Newsletter body as markdown",
            }),
            "A title is required.",
            "empty title",
        ),
        (
            serde_json::json!({
              "title": "a".repeat(71),
              "description": "Newsletter description",
              "content": "## Newsletter body as markdown",
            }),
            "Title exceeds character limit.",
            "excessively large title",
        ),
        (
            serde_json::json!({
              "title": "a</>a",
              "description": "Newsletter description",
              "content": "## Newsletter body as markdown",
            }),
            "Title includes illegal characters.",
            "title with illegal characters",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "a".repeat(201),
              "content": "## Newsletter body as markdown",
            }),
            "Description exceeds character limit.",
            "excessively large description",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "<p>Newsletter description</p>",
              "content": "## Newsletter body as markdown",
            }),
            "Description includes illegal characters.",
            "description with illegal characters",
        ),
    ];

    for (invalid_body, error_message, test_case) in test_cases {
        let response = app.post_admin_create_newsletter(&invalid_body).await;
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
async fn create_newsletter_success_does_not_send_emails() {
    let app = spawn_app().await;

    app.create_confirmed_subscriber(None, None).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter body as markdown",
    }))
    .await;

    app.dispatch_all_pending_emails().await
}
