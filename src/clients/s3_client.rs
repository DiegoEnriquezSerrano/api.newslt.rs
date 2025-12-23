use actix_web::web;
use anyhow::Context;
use s3::BucketConfiguration;
use s3::bucket::Bucket;
use s3::bucket_ops::CannedBucketAcl;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::request::ResponseData;
use uuid::Uuid;

pub struct S3Client {
    pub endpoint: String,
    buckets: Buckets,
}

impl S3Client {
    pub async fn new(
        access_key: Option<String>,
        endpoint: String,
        region: String,
        secret_key: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let buckets =
            Self::initialize_buckets(region, endpoint.clone(), access_key, secret_key).await?;

        Ok(Self { buckets, endpoint })
    }

    pub async fn put_newsletter_issue_cover_image(
        &self,
        newsletter_issue_id: &Uuid,
        content: web::Bytes,
    ) -> Result<ResponseData, anyhow::Error> {
        let path = format!("/newsletter/cover/{newsletter_issue_id}.webp");
        let response: ResponseData = self
            .buckets
            .images
            .put_object_with_content_type(path, &content[..], "image/webp")
            .await
            .context("Failed to store image.")?;

        Ok(response)
    }

    pub async fn put_user_profile_avatar(
        &self,
        user_id: &Uuid,
        content: web::Bytes,
    ) -> Result<ResponseData, anyhow::Error> {
        let path = format!("user/avatar/{user_id}.webp");
        let response: ResponseData = self
            .buckets
            .images
            .put_object_with_content_type(path, &content[..], "image/webp")
            .await
            .context("Failed to store image.")?;

        Ok(response)
    }

    pub async fn put_user_profile_banner(
        &self,
        user_id: &Uuid,
        content: web::Bytes,
    ) -> Result<ResponseData, anyhow::Error> {
        let path = format!("/user/banner/{user_id}.webp");
        let response: ResponseData = self
            .buckets
            .images
            .put_object_with_content_type(path, &content[..], "image/webp")
            .await
            .context("Failed to store image.")?;

        Ok(response)
    }

    async fn initialize_buckets(
        region: String,
        endpoint: String,
        access_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Buckets, anyhow::Error> {
        let mut images_bucket = Bucket::new(
            "images",
            Region::Custom { region, endpoint },
            Credentials {
                access_key: access_key.clone(),
                expiration: None,
                secret_key: secret_key.clone(),
                security_token: None,
                session_token: None,
            },
        )
        .context("Failed to initialize new bucket.")?
        .with_path_style();
        images_bucket = Self::idempotent_bucket_create(images_bucket).await?;

        Ok(Buckets {
            images: images_bucket,
        })
    }

    async fn idempotent_bucket_create(
        mut bucket: Box<Bucket>,
    ) -> Result<Box<Bucket>, anyhow::Error> {
        let bucket_name: &str = &bucket.name();
        let exists: bool = bucket
            .exists()
            .await
            .context(format!("Failed to check if bucket: {bucket_name} exists"))?;
        let credentials = bucket
            .credentials()
            .await
            .context("Failed to get bucket credentials")?;
        let config: BucketConfiguration = BucketConfiguration::new(
            Some(CannedBucketAcl::PublicRead),
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        // Handles for race condition when starting app on multiple threads.
        // e.g. Thread #1 calls 'exists()' and subsequently sends a create request,
        // meanwhile thread #2 calls 'exists()' before thread #1 create completes
        // and subsequently sends its own create request as thread #1 responds.
        if !exists {
            match Bucket::create_with_path_style(bucket_name, bucket.region(), credentials, config)
                .await
            {
                Ok(response) => bucket = response.bucket,
                Err(err) => match err {
                    // https://docs.aws.amazon.com/AmazonS3/latest/API/ErrorResponses.html
                    // <Error><Code>BucketAlreadyOwnedByYou</Code>
                    S3Error::HttpFailWithBody(409, _) => (),
                    _ => anyhow::bail!("Failed to create non-existent bucket."),
                },
            }
        }

        Ok(bucket)
    }
}

pub struct Buckets {
    pub images: Box<Bucket>,
}

#[cfg(test)]
mod tests {
    use crate::clients::s3_client::S3Client;
    use actix_web::body::MessageBody;
    use actix_web::web::Bytes;
    use claims::{assert_err, assert_ok};
    use fake::Fake;
    use fake::faker::lorem::en::Word;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const NONEXISTING_BUCKET_LIST_ALL_RESULT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
      <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
          <Owner>
              <ID>abcd123</ID>
              <DisplayName>test-user</DisplayName>
          </Owner>
          <Buckets>
              <Bucket>
                  <Name>test-bucket</Name>
                  <CreationDate>1969-12-31T23:59:59.999Z</CreationDate>
              </Bucket>
          </Buckets>
      </ListAllMyBucketsResult>"#;

    const EXISTING_BUCKET_LIST_ALL_RESULT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
      <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
          <Owner>
              <ID>abcd123</ID>
              <DisplayName>test-user</DisplayName>
          </Owner>
          <Buckets>
              <Bucket>
                  <Name>images</Name>
                  <CreationDate>1969-12-31T23:59:59.999Z</CreationDate>
              </Bucket>
          </Buckets>
      </ListAllMyBucketsResult>"#;

    fn access_key() -> Option<String> {
        let access_key: String = Word().fake();
        Some(access_key.to_lowercase())
    }

    fn region() -> String {
        let region: String = Word().fake();
        region.to_lowercase()
    }

    fn secret_key() -> Option<String> {
        let secret_key: String = Word().fake();
        Some(secret_key.to_lowercase())
    }

    async fn s3_client(server: &MockServer) -> S3Client {
        S3Client::new(access_key(), server.uri().clone(), region(), secret_key())
            .await
            .unwrap()
    }

    async fn initialize_buckets(mock_server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(NONEXISTING_BUCKET_LIST_ALL_RESULT, "application/xml"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/images"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
    }

    #[tokio::test]
    async fn bucket_endpoint_is_reachable() {
        let mock_server = MockServer::start().await;
        initialize_buckets(&mock_server).await;
        let s3_client: S3Client = s3_client(&mock_server).await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        s3_client
            .buckets
            .images
            .with_path_style()
            .head_object("/")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn does_not_attempt_to_create_bucket_if_already_exists() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(EXISTING_BUCKET_LIST_ALL_RESULT, "application/xml"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/images"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock_server)
            .await;

        s3_client(&mock_server).await;
    }

    #[tokio::test]
    async fn put_newsletter_issue_cover_image_sends_bytes_to_bucket_path() {
        let mock_server = MockServer::start().await;
        initialize_buckets(&mock_server).await;
        let s3_client: S3Client = s3_client(&mock_server).await;

        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let content: Bytes = "Hello, world!".try_into_bytes().unwrap();
        let outcome = s3_client
            .put_newsletter_issue_cover_image(&Uuid::new_v4(), content)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn put_newsletter_issue_cover_image_returns_error_if_unauthorized() {
        s3::set_retries(0);

        let mock_server = MockServer::start().await;
        initialize_buckets(&mock_server).await;
        let s3_client: S3Client = s3_client(&mock_server).await;

        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let content: Bytes = "Hello, world!".try_into_bytes().unwrap();
        let outcome = s3_client
            .put_newsletter_issue_cover_image(&Uuid::new_v4(), content)
            .await;

        assert_err!(outcome);
    }
}
