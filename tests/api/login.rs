use crate::helpers::spawn_app;
use newsletter_api::utils::ResponseErrorMessage;

#[tokio::test]
async fn post_login_responds_with_401_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    // Assert
    assert_eq!(401, response.status().as_u16());

    let response_body: ResponseErrorMessage = response.json().await.unwrap();
    assert_eq!("Authentication failed.", response_body.error);
}
