use crate::authentication::reject_anonymous_users;
use crate::clients::cloudinary_client::CloudinaryClient;
use crate::clients::s3_client::S3Client;
use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    admin, captcha, health_check, index, login, newsletters, subscriptions, users,
};
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
use secrecy::{ExposeSecret, SecretString};
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
        let cloudinary_client = configuration.cloudinary_client.client();
        let s3_client = configuration.s3_client.client().await?;
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
            cloudinary_client,
            email_client,
            s3_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
            host_origin_url,
            configuration.hosts.client,
            configuration.application.session_key,
            configuration.application.captcha_secret,
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
    cloudinary_client: CloudinaryClient,
    email_client: EmailClient,
    s3_client: S3Client,
    base_url: String,
    hmac_secret: SecretString,
    redis_uri: SecretString,
    host_origin_url: String,
    client_url: String,
    session_key: String,
    captcha_secret: SecretString,
) -> Result<Server, anyhow::Error> {
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let client_base_url = Data::new(ApplicationClientBaseUrl(client_url.clone()));
    let cloudinary_client = Data::new(cloudinary_client);
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let s3_client = Data::new(s3_client);
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
                    .service(admin::newsletters::detail::cover_image::put)
                    .service(admin::newsletters::detail::publish::put)
                    .service(admin::user::get)
                    .service(admin::user::put)
                    .service(admin::user::banner::put)
                    .service(admin::user::avatar::put)
                    .service(admin::password::put),
            )
            .service(captcha::get)
            .service(health_check::get)
            .service(login::post)
            .service(newsletters::get)
            .service(newsletters::detail::get)
            .service(newsletters::by_user::get)
            .service(subscriptions::confirm::put)
            .service(subscriptions::post)
            .service(users::detail::get)
            .service(users::get)
            .app_data(client_base_url.clone())
            .app_data(base_url.clone())
            .app_data(cloudinary_client.clone())
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(s3_client.clone())
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
            .app_data(Data::new(CaptchaSecret(captcha_secret.clone())))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 50))
    })
    .listen(listener)?
    .run();
    Ok(server)
}

pub struct ApplicationBaseUrl(pub String);

pub struct ApplicationClientBaseUrl(pub String);

#[derive(Clone)]
pub struct HmacSecret(pub SecretString);

#[derive(Clone)]
pub struct CaptchaSecret(pub SecretString);
