use crate::utils::is_empty_or_whitespace;

#[derive(Debug)]
pub struct Content(String);

impl AsRef<str> for Content {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Content {
    pub fn parse(s: String) -> Result<Content, String> {
        if is_empty_or_whitespace(&s) {
            Err(String::from("Content body is required."))
        } else {
            Ok(Self(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::newsletter_issue::Content;
    use claims::{assert_err, assert_ok};

    #[test]
    fn whitespace_only_descriptions_are_rejected() {
        let content = " ".to_string();

        assert_err!(Content::parse(content));
    }

    #[test]
    fn empty_string_is_rejected() {
        let content = "".to_string();

        assert_err!(Content::parse(content));
    }

    #[test]
    fn a_valid_content_is_parsed_successfully() {
        let content = "Ursula Le Guin".to_string();

        assert_ok!(Content::parse(content));
    }
}
