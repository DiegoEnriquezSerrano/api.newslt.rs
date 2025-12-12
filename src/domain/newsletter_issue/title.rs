use crate::utils::{contains_forbidden_characters, is_empty_or_whitespace, is_too_long};

#[derive(Debug)]
pub struct Title(String);

impl AsRef<str> for Title {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Title {
    pub fn parse(s: String) -> Result<Title, String> {
        if is_empty_or_whitespace(&s) {
            Err(String::from("A title is required."))
        } else if is_too_long(&s, 70) {
            Err(String::from("Title exceeds character limit."))
        } else if contains_forbidden_characters(&s) {
            Err(String::from("Title includes illegal characters."))
        } else {
            Ok(Self(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::newsletter_issue::Title;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_70_grapheme_long_name_is_valid() {
        let name = "ё".repeat(70);

        assert_ok!(Title::parse(name));
    }

    #[test]
    fn a_name_longer_than_70_graphemes_is_rejected() {
        let name = "a".repeat(71);

        assert_err!(Title::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();

        assert_err!(Title::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();

        assert_err!(Title::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();

            assert_err!(Title::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();

        assert_ok!(Title::parse(name));
    }
}
