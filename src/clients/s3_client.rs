use actix_web::web;
use anyhow::Context;
use s3::BucketConfiguration;
use s3::bucket::Bucket;
use s3::bucket_ops::CannedBucketAcl;
use s3::creds::Credentials;
use s3::region::Region;
use s3::request::ResponseData;
use uuid::Uuid;

pub struct S3Client {
    pub buckets: Buckets,
    pub endpoint: String,
    pub region: String,
}

impl S3Client {
    pub async fn new(
        access_key: Option<String>,
        endpoint: String,
        region: String,
        secret_key: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let buckets =
            Self::initialize_buckets(region.clone(), endpoint.clone(), access_key, secret_key)
                .await?;

        Ok(Self {
            buckets,
            endpoint,
            region,
        })
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
        images_bucket = Self::create_if_not_exists(images_bucket).await?;

        Ok(Buckets {
            images: images_bucket,
        })
    }

    async fn create_if_not_exists(mut bucket: Box<Bucket>) -> Result<Box<Bucket>, anyhow::Error> {
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

        if !exists {
            bucket =
                Bucket::create_with_path_style(bucket_name, bucket.region(), credentials, config)
                    .await
                    .context("Failed to get create bucket response.")?
                    .bucket;
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
    use fake::Fake;
    use fake::faker::lorem::en::Word;
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
}
