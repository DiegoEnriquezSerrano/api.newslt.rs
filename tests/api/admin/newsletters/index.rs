use crate::helpers::spawn_app;

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
