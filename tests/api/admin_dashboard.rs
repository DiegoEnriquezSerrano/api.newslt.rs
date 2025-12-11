use crate::helpers::spawn_app;

#[tokio::test]
async fn logout_clears_session_state() {
    // Arrange
    let app = spawn_app().await;

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_eq!(200, response.status().as_u16());

    // Act - Part 2 - Authenticate
    let response = app.get_authenticate().await;
    assert_eq!(200, response.status().as_u16());

    // Act - Part 3 - Logout
    let response = app.post_logout().await;
    assert_eq!(200, response.status().as_u16());

    let response_body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        serde_json::json!({"message": "You have successfully logged out."}),
        response_body
    );

    // Act - Part 4 - Attempt authentication
    let response = app.get_authenticate().await;
    assert_eq!(401, response.status().as_u16());
}
