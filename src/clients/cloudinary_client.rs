use crate::utils::e500;
use actix_web::web;
use anyhow::Context;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};
use urlencoding::encode;
use uuid::Uuid;

pub struct CloudinaryClient {
    pub api_key: String,
    pub api_secret: Secret<String>,
    pub base_url: String,
    pub bucket: String,
    pub http_client: Client,
}

impl CloudinaryClient {
    pub fn new(
        api_key: String,
        api_secret: Secret<String>,
        base_url: String,
        bucket: String,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();

        Self {
            http_client,
            base_url,
            api_key,
            api_secret,
            bucket,
        }
    }

    pub async fn upload_avatar(
        &self,
        file: String,
        user_id: &Uuid,
    ) -> Result<CloudinaryUploadResponse, actix_web::Error> {
        let eager: String = "c_fill,g_auto,ar_1:1,w_450".into();
        let public_id = format!("user/avatar/{}", user_id);
        let transformation: String = "f_webp".into();
        let result = self
            .upload_image(file, public_id, eager, transformation)
            .await
            .map_err(e500)?;

        Ok(result)
    }

    pub async fn upload_banner(
        &self,
        file: String,
        user_id: &Uuid,
    ) -> Result<CloudinaryUploadResponse, actix_web::Error> {
        let eager: String = "c_fill,g_auto,ar_4:1,w_900".into();
        let public_id = format!("user/banner/{}", user_id);
        let transformation: String = "f_webp".into();
        let result = self
            .upload_image(file, public_id, eager, transformation)
            .await
            .map_err(e500)?;

        Ok(result)
    }

    pub async fn upload_newsletter_issue_cover_image(
        &self,
        file: String,
        newsletter_issue_id: &Uuid,
    ) -> Result<CloudinaryUploadResponse, actix_web::Error> {
        let eager: String = "q_70,c_fill,g_auto,ar_16:9,w_1280".into();
        let public_id = format!("newsletter/cover/{}", newsletter_issue_id);
        let transformation: String = "f_webp".into();
        let result = self
            .upload_image(file, public_id, eager, transformation)
            .await
            .map_err(e500)?;

        Ok(result)
    }

    pub async fn upload_image(
        &self,
        file: String,
        public_id: String,
        eager: String,
        transformation: String,
    ) -> Result<CloudinaryUploadResponse, actix_web::Error> {
        let url = format!("{}/v1_1/{}/image/upload", self.base_url, self.bucket);
        let timestamp: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let signed_upload_string = format!(
            "eager={eager}&invalidate=true&public_id={public_id}&timestamp={timestamp}&transformation={transformation}{}",
            self.api_secret.expose_secret()
        );

        let mut hasher = Sha1::new();
        hasher.update(signed_upload_string.as_bytes());
        let result = hasher.finalize();
        let signature = format!("{:x}", result);

        let request_body = UploadImageRequest {
            api_key: self.api_key.clone(),
            eager,
            file,
            public_id,
            signature,
            timestamp,
            transformation,
        };

        let uploaded_image: CloudinaryUploadResponse = self
            .http_client
            .post(&url)
            .body(request_body.to_string())
            .send()
            .await
            .context("Failed to reach image processing service.")
            .map_err(e500)?
            .error_for_status()
            .context("Failed to process image upload.")
            .map_err(e500)?
            .json()
            .await
            .context("Failed to get image upload response.")
            .map_err(e500)?;

        Ok(uploaded_image)
    }

    // We use cloudinary as an image processor to ensure images are formatted
    // as webp and/or to automatically crop/pad images. For the latter we'll
    // have eagerly generated a transformation for the image.
    pub async fn get_image_as_bytes(
        &self,
        uploaded_image: CloudinaryUploadResponse,
    ) -> Result<web::Bytes, actix_web::Error> {
        let secure_url = match uploaded_image.eager {
            Some(e) => e[0].secure_url.clone(),
            None => uploaded_image.secure_url,
        };
        let bytes = self
            .http_client
            .get(secure_url)
            .send()
            .await
            .context("Failed to get image from image server.")
            .map_err(e500)?
            .bytes()
            .await
            .context("Failed to read image bytes.")
            .map_err(e500)?;

        Ok(bytes)
    }
}

#[derive(Serialize, Debug)]
struct UploadImageRequest {
    api_key: String,
    eager: String,
    file: String,
    public_id: String,
    signature: String,
    timestamp: u64,
    transformation: String,
}

impl Display for UploadImageRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "api_key={}&eager={}&file={}&invalidate=true&public_id={}&timestamp={}&transformation={}&signature={}",
            self.api_key,
            self.eager,
            encode(&self.file),
            self.public_id,
            self.timestamp,
            self.transformation,
            self.signature
        )
    }
}

// Struct derived from Cloudinary upload API documentation
// https://cloudinary.com/documentation/upload_images#upload_response
#[derive(Serialize, Deserialize, Debug)]
pub struct CloudinaryUploadResponse {
    pub eager: Option<Vec<EagerTransformation>>,
    pub secure_url: String,
    api_key: String,
    asset_folder: String,
    asset_id: String,
    bytes: u32,
    created_at: String,
    display_name: String,
    etag: String,
    format: String,
    height: u32,
    overwritten: Option<bool>,
    pages: u32,
    placeholder: bool,
    public_id: String,
    resource_type: String,
    signature: String,
    url: String,
    version_id: String,
    version: u32,
    width: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EagerTransformation {
    pub secure_url: String,
    bytes: u32,
    format: String,
    height: u32,
    transformation: String,
    url: String,
    width: u32,
}

#[cfg(test)]
mod tests {
    use crate::clients::cloudinary_client::fixtures::mock_cloudinary_upload_response;
    use crate::clients::cloudinary_client::{CloudinaryClient, CloudinaryUploadResponse};
    use claims::{assert_err, assert_ok};
    use fake::faker::lorem::en::Word;
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct UploadImageBodyMatcher;

    impl wiremock::Match for UploadImageBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let body = String::from_utf8(request.body.clone()).unwrap();

            body.contains("api_key=")
                && body.contains("eager=")
                && body.contains("file=")
                && body.contains("public_id=")
                && body.contains("timestamp=")
                && body.contains("transformation=")
                && body.contains("signature=")
        }
    }

    fn bucket() -> String {
        let bucket: String = Word().fake();
        let bucket = bucket.to_lowercase();
        bucket
    }

    fn public_id() -> String {
        let public_id: String = Word().fake();
        let public_id = public_id.to_lowercase();
        public_id
    }

    fn file() -> String {
        String::from(
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==",
        )
    }

    fn eager() -> String {
        String::from("w_400,h_300,c_pad|w_260,h_200,c_crop")
    }

    fn transformation() -> String {
        String::from("f_webp")
    }

    fn cloudinary_client(base_url: String) -> CloudinaryClient {
        CloudinaryClient::new(
            Faker.fake(),
            Secret::new(Faker.fake()),
            base_url,
            bucket(),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn upload_image_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let cloudinary_client = cloudinary_client(mock_server.uri());

        Mock::given(path(format!(
            "/v1_1/{}/image/upload",
            cloudinary_client.bucket
        )))
        .and(method("POST"))
        .and(UploadImageBodyMatcher)
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

        let _ = cloudinary_client
            .upload_image(file(), public_id(), eager(), transformation())
            .await;
    }

    #[tokio::test]
    async fn upload_image_returns_ok_response() {
        let mock_server = MockServer::start().await;
        let cloudinary_client = cloudinary_client(mock_server.uri().clone());

        Mock::given(any())
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_cloudinary_upload_response(&mock_server.uri())),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = cloudinary_client
            .upload_image(file(), public_id(), eager(), transformation())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn upload_image_fails_if_server_is_unreachable() {
        let cloudinary_client =
            cloudinary_client(String::from("http://127.0.0.1:99999/nonexistentpath"));
        let outcome = cloudinary_client
            .upload_image(file(), public_id(), eager(), transformation())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn upload_image_fails_if_response_is_unauthorized() {
        let mock_server = MockServer::start().await;
        let cloudinary_client = cloudinary_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = cloudinary_client
            .upload_image(file(), public_id(), eager(), transformation())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn upload_image_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let cloudinary_client = cloudinary_client(mock_server.uri());
        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = cloudinary_client
            .upload_image(file(), public_id(), eager(), transformation())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn get_image_as_bytes_fires_a_request_to_cloudinary_server() {
        let mock_server = MockServer::start().await;
        let cloudinary_client = cloudinary_client(mock_server.uri().clone());

        Mock::given(method("GET"))
            .and(path("/cld-docs/image/upload/v1719307544/c_fill,g_auto,h_450,w_450/gotjephlnz2jgiu20zni.webp"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"Hello, world!"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response: CloudinaryUploadResponse = serde_json::from_str(
            mock_cloudinary_upload_response(&mock_server.uri())
                .to_string()
                .as_str(),
        )
        .unwrap();

        let outcome = cloudinary_client.get_image_as_bytes(response).await;

        assert_ok!(outcome);
    }
}

pub mod fixtures {
    pub fn mock_cloudinary_upload_response(base_url: &String) -> serde_json::Value {
        serde_json::json!({
          "eager": [{
            "bytes": 27910,
            "format": "webp",
            "height": 450,
            "secure_url": format!("{}/cld-docs/image/upload/v1719307544/c_fill,g_auto,h_450,w_450/gotjephlnz2jgiu20zni.webp", base_url),
            "transformation": "c_fill,g_auto,h_450,w_450",
            "url": format!("{}/cld-docs/image/upload/v1719307544/c_fill,g_auto,h_450,w_450/gotjephlnz2jgiu20zni.webp", base_url),
            "width": 450,
          }],
          "api_key": "614335564976464",
          "asset_folder": "",
          "asset_id": "3515c6000a548515f1134043f9785c2f",
          "bytes": 896838,
          "created_at": "2024-06-25T09:25:44Z",
          "display_name": "gotjephlnz2jgiu20zni",
          "etag": "2a2df1d2d2c3b675521e866599273083",
          "format": "webp",
          "height": 1441,
          "original_filename": "sample",
          "overwritten": true,
          "pages": 1,
          "placeholder": false,
          "public_id": "gotjephlnz2jgiu20zni",
          "resource_type": "image",
          "secure_url": format!("{}/cld-docs/image/upload/v1719307544/gotjephlnz2jgiu20zni.webp", base_url),
          "signature": "d0b1009e3271a942836c25756ce3e04d205bf754",
          "tags": [],
          "type": "upload",
          "url": format!("{}/cld-docs/image/upload/v1719307544/gotjephlnz2jgiu20zni.webp", base_url),
          "version_id": "7d2cc533bee9ff39f7da7414b61fce7e",
          "version": 1719307544,
          "width": 1920,
        })
    }
}
