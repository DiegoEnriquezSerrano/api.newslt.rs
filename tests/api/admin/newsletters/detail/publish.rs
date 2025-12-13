use crate::helpers::spawn_app;
use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use newsletter_api::models::{NewUser, NewUserData, NewsletterIssueAPI};
use newsletter_api::utils::{ResponseErrorMessage, ResponseMessage};
use secrecy::Secret;
use std::time::Duration;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn publish_newsletters_returns_400_for_invalid_idempotency_keys() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "<p>Newsletter body as HTML</p>",
    }))
    .await;

    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;

    let test_cases = vec![
        (
            serde_json::json!({
              "idempotency_key": ""
            }),
            "empty key",
        ),
        (
            serde_json::json!({
              "idempotency_key": "a".repeat(51)
            }),
            "key is too large",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app
            .put_admin_publish_newsletter(&newsletter_issue_id, &invalid_body)
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
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.create_confirmed_subscriber(None, None).await;
    app.test_user.login(&app).await;

    // Act - Part 1 - Submit newsletter form
    let response = app
        .post_admin_create_newsletter(&serde_json::json!({
          "title": "Newsletter title",
          "description": "Newsletter description",
          "content": "## Newsletter body as markdown",
        }))
        .await;
    assert_eq!(201, response.status().as_u16());

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
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
    assert_eq!(
        "The newsletter issue has been accepted - emails will go out shortly.".to_string(),
        response.json::<ResponseMessage>().await.unwrap().message
    );

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.create_unconfirmed_subscriber(None, None).await;
    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "description": "Newsletter description as plain text",
        "content": "## Newsletter body as markdown",
    });
    let response = app
        .post_admin_create_newsletter(&newsletter_request_body)
        .await;
    assert_eq!(201, response.status().as_u16());

    let response_body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        serde_json::json!({ "message": "The newsletter issue has been created.".to_string() }),
        response_body
    );

    // Act - Part 2 - Publish newsletter form
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
    assert_eq!(
        "The newsletter issue has been accepted - emails will go out shortly.".to_string(),
        response.json::<ResponseMessage>().await.unwrap().message
    );

    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletter_publishing_is_idempotent() {
    // Arrange
    let app = spawn_app().await;
    app.create_confirmed_subscriber(None, None).await;
    app.test_user.login(&app).await;

    // Act - Part 1 - Submit newsletter form
    let response = app
        .post_admin_create_newsletter(&serde_json::json!({
          "title": "Newsletter title",
          "description": "Newsletter description",
          "content": "## Newsletter body as markdown",
        }))
        .await;
    assert_eq!(201, response.status().as_u16());

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 2 - Publish newsletter
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;
    let publish_request_body = serde_json::json!({
      "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = app
        .put_admin_publish_newsletter(&newsletter_issue_id, &publish_request_body)
        .await;
    assert_eq!(200, response.status().as_u16());

    let expected_message =
        "The newsletter issue has been accepted - emails will go out shortly.".to_string();
    assert_eq!(
        &expected_message,
        &response.json::<ResponseMessage>().await.unwrap().message
    );

    // Act - Part 3 - Submit newsletter form **again**
    let response = app
        .put_admin_publish_newsletter(&newsletter_issue_id, &publish_request_body)
        .await;
    assert_eq!(200, response.status().as_u16());
    assert_eq!(
        &expected_message,
        &response.json::<ResponseMessage>().await.unwrap().message
    );

    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we have sent the newsletter email **once**
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // Arrange
    let app = spawn_app().await;
    app.create_confirmed_subscriber(None, None).await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "<p>Newsletter body as HTML</p>",
    }))
    .await;

    Mock::given(path("/email"))
        .and(method("POST"))
        // Setting a long delay to ensure that the second request
        // arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Submit two newsletter forms concurrently
    let response = app.get_admin_unpublished_newsletter_issues().await;
    let response_body: Vec<NewsletterIssueAPI> = response.json().await.unwrap();
    let newsletter_issue_id = response_body[0].newsletter_issue_id;
    let newsletter_request_body = serde_json::json!({
      "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response1 =
        app.put_admin_publish_newsletter(&newsletter_issue_id, &newsletter_request_body);
    let response2 =
        app.put_admin_publish_newsletter(&newsletter_issue_id, &newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we have sent the newsletter email **once**
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_anothers_confirmed_subscribers() {
    let app = spawn_app().await;
    let subscriber_email: String = SafeEmail().fake();
    let second_user: NewUser = NewUserData {
        username: uuid::Uuid::new_v4().to_string(),
        email: SafeEmail().fake(),
        password: Secret::from(uuid::Uuid::new_v4().to_string()),
    }
    .try_into()
    .unwrap();

    let mut transaction = app.db_pool.begin().await.unwrap();
    let second_user = second_user.store(&mut transaction).await.unwrap();
    let _ = transaction.commit().await;

    app.create_confirmed_subscriber(Some(second_user.user_id), Some(subscriber_email.clone()))
        .await;
    app.create_confirmed_subscriber(None, Some(subscriber_email.clone()))
        .await;

    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "<p>Newsletter body as HTML</p>",
    }))
    .await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
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
    assert_eq!(
        "The newsletter issue has been accepted - emails will go out shortly.".to_string(),
        response.json::<ResponseMessage>().await.unwrap().message
    );

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn publish_newsletters_returns_422_for_empty_description() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "",
      "content": "<p>Newsletter body as HTML</p>",
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
    assert_eq!(400, response.status().as_u16());

    let response_body: ResponseErrorMessage = response.json().await.unwrap();
    assert_eq!(
        "A description is required.".to_string(),
        response_body.error
    );
}

#[tokio::test]
async fn publish_newsletters_returns_422_for_empty_content() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    app.post_admin_create_newsletter(&serde_json::json!({
      "title": "Newsletter title",
      "description": "Newsletter description",
      "content": "",
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
    assert_eq!(400, response.status().as_u16());

    let response_body: ResponseErrorMessage = response.json().await.unwrap();
    assert_eq!("Content body is required.".to_string(), response_body.error);
}
