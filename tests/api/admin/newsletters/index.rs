use crate::helpers::spawn_app;
use newsletter_api::clients::cloudinary_client::fixtures::mock_cloudinary_upload_response;
use newsletter_api::models::NewsletterIssueAPI;
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
          "cover_image": "",
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
          "cover_image": "",
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
              "cover_image": "",
            }),
            "missing title",
        ),
        (
            serde_json::json!({
              "title": "Newsletter!",
              "description": "Newsletter description",
              "cover_image": "",
            }),
            "missing content",
        ),
        (
            serde_json::json!({
              "title": "Newsletter!",
              "content": "<p>Newsletter body as HTML</p>",
              "cover_image": "",
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
              "cover_image": "",
            }),
            "A title is required.",
            "empty title",
        ),
        (
            serde_json::json!({
              "title": "a".repeat(71),
              "description": "Newsletter description",
              "content": "## Newsletter body as markdown",
              "cover_image": "",
            }),
            "Title exceeds character limit.",
            "excessively large title",
        ),
        (
            serde_json::json!({
              "title": "a</>a",
              "description": "Newsletter description",
              "content": "## Newsletter body as markdown",
              "cover_image": "",
            }),
            "Title includes illegal characters.",
            "title with illegal characters",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "a".repeat(201),
              "content": "## Newsletter body as markdown",
              "cover_image": "",
            }),
            "Description exceeds character limit.",
            "excessively large description",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
              "description": "<p>Newsletter description</p>",
              "content": "## Newsletter body as markdown",
              "cover_image": "",
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
      "cover_image": "",
    }))
    .await;

    app.dispatch_all_pending_emails().await
}

#[tokio::test]
async fn create_newsletter_can_process_image() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    let mock_response = mock_cloudinary_upload_response(&app.cloudinary_server.uri());

    Mock::given(path(format!(
        "/v1_1/{}/image/upload",
        &app.cloudinary_client.bucket
    )))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
    .expect(1)
    .mount(&app.cloudinary_server)
    .await;

    // Act 1 - Create newsletter issue with cover image data url
    let response = app
        .post_admin_create_newsletter(&serde_json::json!({
          "title": "Newsletter title",
          "description": "Newsletter description",
          "content": "## Newsletter body as markdown",
          "cover_image": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg=="
        }))
        .await;

    assert_eq!(201, response.status().as_u16());
    assert_eq!(
        String::from("The newsletter issue has been created."),
        response.json::<ResponseMessage>().await.unwrap().message
    );

    // Act 2 - Check that record is created with cover image url set
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;
    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    let response_body: NewsletterIssueAPI = response.json().await.unwrap();

    assert!(response_body.cover_image_url.contains(&format!(
        "/images/newsletter/cover/{newsletter_issue_id}.webp",
    )));
}
