use crate::utils::is_empty_or_whitespace;
use std::path::Path;
use validator::ValidateUrl;

#[derive(Debug)]
pub struct ImageUrl(String);

impl AsRef<str> for ImageUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ImageUrl {
    pub fn parse(s: String) -> Result<ImageUrl, String> {
        let s = s.trim().to_string();

        if is_empty_or_whitespace(&s) {
            return Ok(Self(s));
        }

        if !ValidateUrl::validate_url(&s) {
            return Err(format!("{} is not a valid url.", s));
        }

        has_image_file_extension(&s)?;

        Ok(Self(s))
    }
}

impl std::fmt::Display for ImageUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

fn has_image_file_extension(url: &String) -> Result<(), String> {
    let valid_image_extensions = ["png", "jpg", "jpeg", "webp", "avif", "gif"];
    let path = Path::new(&url);
    let ext = path.extension();

    match ext {
        Some(e) => {
            if valid_image_extensions
                .iter()
                .any(|img_ext| e.to_str().unwrap() == *img_ext)
            {
                Ok(())
            } else {
                Err(format!("File extension '{:?}' is not valid.", e))
            }
        }
        None => Err(String::from("No file extension found.")),
    }
}

#[cfg(test)]
mod tests {
    use super::ImageUrl;
    use claims::{assert_err, assert_ok};

    #[test]
    fn empty_string_is_accepted() {
        let url = "".to_string();

        assert_ok!(ImageUrl::parse(url));
    }

    #[test]
    fn url_missing_file_extension_is_rejected() {
        let url = "https://example.com/image".to_string();

        assert_err!(ImageUrl::parse(url));
    }

    #[test]
    fn url_with_invalid_file_extension_is_rejected() {
        let url = "https://example.com/image.csv".to_string();

        assert_err!(ImageUrl::parse(url));
    }

    #[test]
    fn imgae_url_with_query_params_is_invalid() {
        let url = String::from("https://example.com/image.png?w=100");

        assert_err!(ImageUrl::parse(url));
    }

    #[test]
    fn valid_urls_are_parsed_successfully() {
        let url = String::from("https://example.com/image.png");

        assert_ok!(ImageUrl::parse(url));
    }
}
