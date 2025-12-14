use crate::utils::{contains_forbidden_characters, is_too_long};

#[derive(Debug)]
pub struct DisplayName(String);

impl AsRef<str> for DisplayName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl DisplayName {
    pub fn parse(s: String) -> Result<DisplayName, String> {
        if is_too_long(&s, 70) {
            Err(String::from("Display name exceeds character limit."))
        } else if contains_forbidden_characters(&s) {
            Err(String::from("Display name includes illegal characters."))
        } else {
            Ok(Self(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::user_profile::DisplayName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_70_grapheme_long_name_is_valid() {
        let name = "ё".repeat(70);

        assert_ok!(DisplayName::parse(name));
    }

    #[test]
    fn a_name_longer_than_70_graphemes_is_rejected() {
        let name = "a".repeat(71);

        assert_err!(DisplayName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();

            assert_err!(DisplayName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();

        assert_ok!(DisplayName::parse(name));
    }
}
