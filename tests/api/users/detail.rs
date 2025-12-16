use crate::helpers::spawn_app;
use newsletter_api::models::PublicProfile;

#[tokio::test]
async fn nonexisting_username_returns_not_found() {
    let app = spawn_app().await;

    let response = app.get_user(&"slappy_white".to_string()).await;
    assert_eq!(404, response.status().as_u16());
}

#[tokio::test]
async fn existing_username_return_user() {
    let app = spawn_app().await;

    let response = app.get_user(&app.test_user.username).await;
    assert_eq!(200, response.status().as_u16());

    let response_body: PublicProfile = response.json().await.unwrap();
    assert_eq!(&response_body.username, &app.test_user.username);
    assert_eq!(&response_body.description, "");
    assert_eq!(&response_body.display_name, "");
}
