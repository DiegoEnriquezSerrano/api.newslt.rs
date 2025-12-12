use crate::utils::{is_empty_or_whitespace, is_too_long};
use regex::Regex;

#[derive(Debug)]
pub struct Username(String);

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Username {
    pub fn parse(s: String) -> Result<Username, String> {
        let expression = Regex::new(r"^[\w.~-]*$").unwrap();
        let contains_forbidden_characters = !expression.is_match(&s);

        if is_empty_or_whitespace(&s) || is_too_long(&s, 70) || contains_forbidden_characters {
            Err(format!("{} is not a valid username.", s))
        } else {
            Ok(Self(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::user::Username;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_70_grapheme_long_name_is_valid() {
        let name = "ё".repeat(70);

        assert_ok!(Username::parse(name));
    }

    #[test]
    fn a_name_longer_than_70_graphemes_is_rejected() {
        let name = "a".repeat(71);

        assert_err!(Username::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();

        assert_err!(Username::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();

        assert_err!(Username::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        let name = "my%dude".to_string();

        assert_err!(Username::parse(name));
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula-Le-Guin".to_string();

        assert_ok!(Username::parse(name));
    }
}
