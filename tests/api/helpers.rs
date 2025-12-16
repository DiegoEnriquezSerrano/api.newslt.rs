use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use newsletter_api::configuration::{DatabaseSettings, get_configuration};
use newsletter_api::email_client::EmailClient;
use newsletter_api::issue_delivery_worker::{ExecutionOutcome, try_execute_task};
use newsletter_api::models::{NewUser, NewUserData, UserProfile};
use newsletter_api::startup::{Application, get_connection_pool};
use newsletter_api::telemetry::{get_subscriber, init_subscriber};
use secrecy::Secret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }

    pub async fn post_subscriptions<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn put_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .put(&format!("{}/admin/password", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_newsletter_issues(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_unpublished_newsletter_issues(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/newsletters/drafts", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_newsletter_issue(
        &self,
        newsletter_issue_id: &Uuid,
    ) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{}/admin/newsletters/{}",
                &self.address, newsletter_issue_id
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_admin_create_newsletter<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/newsletters", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn put_admin_publish_newsletter<Body>(
        &self,
        newsletter_issue_id: &Uuid,
        body: &Body,
    ) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .put(&format!(
                "{}/admin/newsletter/{}/publish",
                &self.address, newsletter_issue_id
            ))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn put_admin_update_newsletter<Body>(
        &self,
        newsletter_issue_id: &Uuid,
        body: &Body,
    ) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .put(&format!(
                "{}/admin/newsletters/{}",
                &self.address, newsletter_issue_id
            ))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_user(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/user", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn put_admin_update_user<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .put(&format!("{}/admin/user", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_public_newsletters(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_public_newsletter(
        &self,
        username: &String,
        slug: &String,
    ) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{}/newsletters/by_user/{}/issue/{}",
                &self.address, username, slug
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_public_newsletters_by_user(&self, username: &String) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{}/newsletters/by_user/{}",
                &self.address, username
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_users(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/users", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_user(&self, username: &String) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/users/{}", &self.address, username))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_authenticate(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/authenticate", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn create_unconfirmed_subscriber(
        &self,
        user_id: Option<Uuid>,
        email: Option<String>,
    ) -> ConfirmationLinks {
        // We are working with multiple subscribers now,
        // their details must be randomised to avoid conflicts!
        let name: String = Name().fake();
        let email: String = email.unwrap_or(SafeEmail().fake());
        let user_id: Uuid = user_id.unwrap_or(self.test_user.user_id);
        let body = &serde_json::json!({
            "name": name,
            "email": email,
            "user_id": user_id
        });

        let _mock_guard = Mock::given(path("/email"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .named("Create unconfirmed subscriber")
            .expect(1)
            .mount_as_scoped(&self.email_server)
            .await;
        self.post_subscriptions(body)
            .await
            .error_for_status()
            .unwrap();

        let email_request = self
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();
        self.get_confirmation_links(&email_request)
    }

    pub async fn create_confirmed_subscriber(&self, user_id: Option<Uuid>, email: Option<String>) {
        let confirmation_link = self.create_unconfirmed_subscriber(user_id, email).await;

        self.api_client
            .put(confirmation_link.html)
            .send()
            .await
            .expect("Failed to confirm subscriber.");
    }

    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    // Launch a mock server to stand in for Postmark's API
    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        c.application.port = 0;
        // Use the mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // Launch the application as a background task
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let db_pool = get_connection_pool(&configuration.database);
    let test_user = TestUser::create(&db_pool)
        .await
        .expect("Failed to create test user.");
    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool,
        email_server,
        test_user,
        api_client: client,
        email_client: configuration.email_client.client(),
    };

    test_app
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: Secret::new("password".to_string()),
        ..config.clone()
    };
    let mut connection = PgConnection::connect_with(&maintenance_settings.connect_options())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

pub struct TestUser {
    pub username: String,
    pub password: String,
    pub user_id: Uuid,
}

impl TestUser {
    pub async fn create(pool: &PgPool) -> Result<Self, &str> {
        let password: String = Uuid::new_v4().to_string();
        let new_user: NewUser = NewUserData {
            username: Uuid::new_v4().to_string(),
            password: Secret::from(password.clone()),
            email: SafeEmail().fake(),
        }
        .try_into()
        .expect("Failed to initialize new user.");
        let mut transaction = pool
            .begin()
            .await
            .expect("Failed to begin database transaction.");
        let new_user = new_user
            .store(&mut transaction)
            .await
            .expect("Failed to store test user.");
        UserProfile::initialize(&new_user.user_id)
            .insert(&mut transaction)
            .await
            .unwrap();
        transaction
            .commit()
            .await
            .expect("Failed to commit database transaction.");

        Ok(Self {
            username: new_user.username,
            password,
            user_id: new_user.user_id,
        })
    }

    pub async fn login(&self, app: &TestApp) {
        app.post_login(&serde_json::json!({
            "username": &self.username,
            "password": &self.password
        }))
        .await;
    }
}
