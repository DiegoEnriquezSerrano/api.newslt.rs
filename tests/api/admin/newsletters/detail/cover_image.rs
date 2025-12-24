use crate::helpers::spawn_app;
use newsletter_api::clients::cloudinary_client::fixtures::mock_cloudinary_upload_response;
use newsletter_api::models::NewsletterIssueAPI;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn authenticated_user_can_update_a_newsletters_cover_image() {
    let app = spawn_app().await;
    let mock_response = mock_cloudinary_upload_response(&app.cloudinary_server.uri());

    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "## Newsletter content",
      "cover_image": ""
    }))
    .await;

    Mock::given(path(format!(
        "/v1_1/{}/image/upload",
        &app.cloudinary_client.bucket
    )))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
    .expect(1)
    .mount(&app.cloudinary_server)
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    let response = app
        .put_admin_update_newsletter_issue_cover_image(
            &newsletter_issue_id,
            &serde_json::json!({
              "image": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg=="
            }),
        )
        .await;
    assert_eq!(200, response.status().as_u16());

    let response = app.get_admin_newsletter_issue(&newsletter_issue_id).await;
    let response_body: NewsletterIssueAPI = response.json().await.unwrap();

    assert!(response_body.cover_image_url.contains(&format!(
        "/images/newsletter/cover/{newsletter_issue_id}.webp"
    )));
}
