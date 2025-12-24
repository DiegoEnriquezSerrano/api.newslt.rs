use crate::helpers::spawn_app;
use newsletter_api::clients::cloudinary_client::fixtures::mock_cloudinary_upload_response;
use newsletter_api::models::UserProfileAPI;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn authenticated_user_can_update_profile_banner() {
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

    let response = app
        .put_admin_update_user_profile_banner(&serde_json::json!({
          "image": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==",
        }))
        .await;

    assert_eq!(200, response.status().as_u16());

    let response = app.get_admin_user().await;
    let response_body: UserProfileAPI = response.json().await.unwrap();

    assert!(response_body.banner_url.contains(&format!(
        "/images/user/banner/{}.webp",
        app.test_user.user_id
    )));
}
