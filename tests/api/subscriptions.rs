use crate::helpers::spawn_app;
use claims::assert_ok;
use newsletter_api::utils::ResponseErrorMessage;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_params() {
    // Arrange
    let app = spawn_app().await;

    Mock::given(path("/api/v1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let response = app
        .post_subscriptions(
            &serde_json::json!({"name": "le guin", "email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // Arrange
    let app = spawn_app().await;

    // Act
    app.post_subscriptions(
        &serde_json::json!({"name": "le guin", "email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
    )
    .await;

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_subscriptions(
            &serde_json::json!({"name": "le guin", "email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
        )
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;

    Mock::given(path("/api/v1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(
        &serde_json::json!({"name": "le guin", "email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
    )
    .await;

    // Assert
    // Mock asserts on drop
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;

    Mock::given(path("/api/v1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(
        &serde_json::json!({"name": "le guin", "email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
    )
    .await;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({"name": "le guin", "username": &app.test_user.username}),
            "missing the email",
        ),
        (
            serde_json::json!({"email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
            "missing the email",
        ),
        (
            serde_json::json!({"username": &app.test_user.username}),
            "missing both name and email",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(&invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_with_json_message_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({"name": "", "email": "ursula_le_guin@gmail.com", "username": &app.test_user.username}),
            "empty name",
        ),
        (
            serde_json::json!({"name": "Ursula", "email": "", "username": &app.test_user.username}),
            "empty email",
        ),
        (
            serde_json::json!({"name": "Ursula", "email": "definitely-not-an-email", "username": &app.test_user.username}),
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(&body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );

        let response_body: Result<ResponseErrorMessage, reqwest::Error> = response.json().await;

        assert_ok!(response_body);
    }
}
