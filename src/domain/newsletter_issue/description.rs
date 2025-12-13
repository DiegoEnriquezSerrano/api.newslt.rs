use crate::utils::{contains_forbidden_characters, is_empty_or_whitespace, is_too_long};

#[derive(Debug)]
pub struct Description(String);

impl AsRef<str> for Description {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Description {
    pub fn parse(s: String) -> Result<Description, String> {
        if is_empty_or_whitespace(&s) {
            Err(String::from("A description is required."))
        } else if is_too_long(&s, 200) {
            Err(String::from("Description exceeds character limit."))
        } else if contains_forbidden_characters(&s) {
            Err(String::from("Description includes illegal characters."))
        } else {
            Ok(Self(s))
        }
    }

    pub fn parse_draft(s: String) -> Result<Description, String> {
        if is_too_long(&s, 200) {
            Err(String::from("Description exceeds character limit."))
        } else if contains_forbidden_characters(&s) {
            Err(String::from("Description includes illegal characters."))
        } else {
            Ok(Self(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::newsletter_issue::Description;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_200_grapheme_long_description_is_valid() {
        let description = "ё".repeat(200);

        assert_ok!(Description::parse(description));
    }

    #[test]
    fn a_description_longer_than_200_graphemes_is_rejected() {
        let description = "a".repeat(201);

        assert_err!(Description::parse(description));
    }

    #[test]
    fn whitespace_only_descriptions_are_rejected() {
        let description = " ".to_string();

        assert_err!(Description::parse(description));
    }

    #[test]
    fn empty_string_is_rejected() {
        let description = "".to_string();

        assert_err!(Description::parse(description));
    }

    #[test]
    fn descriptions_containing_an_invalid_character_are_rejected() {
        for description in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let description = description.to_string();

            assert_err!(Description::parse(description));
        }
    }

    #[test]
    fn a_valid_description_is_parsed_successfully() {
        let description = "Ursula Le Guin".to_string();

        assert_ok!(Description::parse(description));
    }
}
