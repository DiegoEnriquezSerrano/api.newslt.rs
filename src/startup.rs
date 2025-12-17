use crate::authentication::reject_anonymous_users;
use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{admin, health_check, index, login, newsletters, subscriptions, users};
use actix_cors::Cors;
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::middleware::from_fn;
use actix_web::web::Data;
use actix_web::{App, HttpServer, web};
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let email_client = configuration.email_client.client();
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let host_origin_url = format!("{}:{}", configuration.application.host, port);
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
            host_origin_url,
            configuration.hosts.client,
            configuration.application.session_key,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.connect_options())
}

#[allow(clippy::too_many_arguments)]
async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
    host_origin_url: String,
    client_url: String,
    session_key: String,
) -> Result<Server, anyhow::Error> {
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(client_url.as_str())
            .allowed_origin(host_origin_url.as_str())
            .allowed_methods(vec![
                "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD",
            ])
            .allowed_headers(&[
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
            ])
            .expose_headers(&[
                actix_web::http::header::CONTENT_DISPOSITION,
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::EXPIRES,
            ])
            .supports_credentials()
            .block_on_origin_mismatch(true)
            .max_age(3600);
        let session_middleware =
            SessionMiddleware::builder(redis_store.clone(), secret_key.clone())
                .cookie_name(session_key.clone())
                .build();

        App::new()
            .wrap(message_framework.clone())
            .wrap(session_middleware)
            .wrap(cors)
            .wrap(TracingLogger::default())
            .service(index::get)
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_users))
                    .service(admin::authenticate::get)
                    .service(admin::logout::post)
                    .service(admin::newsletters::get)
                    .service(admin::newsletters::post)
                    .service(admin::newsletters::drafts::get)
                    .service(admin::newsletters::detail::get)
                    .service(admin::newsletters::detail::put)
                    .service(admin::newsletters::detail::publish::put)
                    .service(admin::user::get)
                    .service(admin::user::put)
                    .service(admin::password::put),
            )
            .service(health_check::get)
            .service(login::post)
            .service(newsletters::get)
            .service(newsletters::detail::get)
            .service(newsletters::by_user::get)
            .service(subscriptions::confirm::put)
            .service(subscriptions::post)
            .service(users::detail::get)
            .service(users::get)
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();
    Ok(server)
}

pub struct ApplicationBaseUrl(pub String);

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);
