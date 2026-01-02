use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use anyhow::Context;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use captcha::Captcha;
use captcha::filters::{Dots, Grid, Noise, Wave};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Base64Challenger {
    pub base64_image: String,
    answer: String,
    secret: Secret<String>,
}

impl Base64Challenger {
    pub fn new(secret: Secret<String>) -> Result<Self, anyhow::Error> {
        if secret.expose_secret().as_bytes().len() != 32 {
            anyhow::bail!("Secret must be 32 bytes in length.")
        }

        let mut captcha = Captcha::new();
        captcha
            .add_chars(6)
            .view(320, 120)
            .set_color([23, 23, 23])
            .apply_filter(Dots::new(10))
            .apply_filter(Wave::new(1.5, 40.0))
            .apply_filter(Noise::new(0.1))
            .apply_filter(Grid::new(20, 10));
        let answer = captcha.chars_as_string();
        let base64_image = captcha
            .as_base64()
            .context("Failed to generate base64 string from image.")?;

        Ok(Self {
            answer,
            base64_image,
            secret,
        })
    }

    pub fn encrypt(&self) -> Result<String, anyhow::Error> {
        // Create cipher from 32-byte key. (Panics if key is not 32 bytes)
        let key = Key::<Aes256Gcm>::from_slice(self.secret.expose_secret().as_bytes());
        let cipher = Aes256Gcm::new(key);
        // Nonce is a 12-byte value.
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = match cipher.encrypt(&nonce, self.answer.as_bytes()) {
            Ok(cipher) => cipher,
            Err(_) => anyhow::bail!("Failed to encrypt challenge."),
        };

        // Base64 encode nonce + ciphertext for output.
        let mut out = Vec::with_capacity(nonce.as_slice().len() + ciphertext.len());
        out.extend_from_slice(nonce.as_slice());
        out.extend_from_slice(&ciphertext);

        Ok(STANDARD.encode(out))
    }

    pub fn decrypt(encoded: &str, secret: Secret<String>) -> Result<String, anyhow::Error> {
        let data = STANDARD
            .decode(encoded)
            .context("Failed to base64 decode challenge.")?;

        if data.len() < 12 {
            anyhow::bail!("Ciphertext is too short.")
        }

        // Split nonce and ciphertext for decryption.
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let key = Key::<Aes256Gcm>::from_slice(secret.expose_secret().as_bytes());
        let cipher = Aes256Gcm::new(key);

        match cipher.decrypt(nonce, ciphertext.as_ref()) {
            Ok(bytes_vec) => {
                String::from_utf8(bytes_vec).context("Failed to convert answer to string.")
            }
            Err(_) => anyhow::bail!("Failed to decrypt answer."),
        }
    }

    pub fn verify(
        encoded: &str,
        answer: String,
        secret: Secret<String>,
    ) -> Result<(), anyhow::Error> {
        if Self::decrypt(encoded, secret)? == answer {
            Ok(())
        } else {
            anyhow::bail!("Incorrect answer.")
        }
    }
}

#[derive(Serialize, Debug)]
pub struct CaptchaResponse {
    pub challenge_image: String,
    pub challenge: String,
}

#[cfg(test)]
mod tests {
    use crate::challenge::Base64Challenger;
    use claims::{assert_err, assert_ok};
    use secrecy::Secret;

    #[test]
    fn secret_must_be_32_bytes_in_length() {
        let secret = Secret::from("W81lMp7E1J0569L2Z1ERpeX8XDiYn11".to_string());
        let challenge = Base64Challenger::new(secret);

        assert_err!(challenge);
    }

    #[test]
    fn can_create_challenge_image() {
        let secret = Secret::from("w8ar9i496zulwEayDG828Y67i09IfwWC".to_string());
        let challenge = Base64Challenger::new(secret);

        assert_ok!(challenge);
    }

    #[test]
    fn can_encrypt_challenge() {
        let secret = Secret::from("njE17BV5QLYO82V3UWoa22ZwwdiD40l2".to_string());
        let challenge = Base64Challenger::new(secret).expect("Creating challenge.");

        assert_ok!(challenge.encrypt());
    }

    #[test]
    fn can_decrypt_challenge() {
        let secret = Secret::from("tNuS550e9os25IFZxw518GlNSK3ouiY1".to_string());
        let challenge = Base64Challenger::new(secret).expect("Creating challenge.");
        let encrypted = challenge.encrypt().unwrap();
        let decrypted = Base64Challenger::decrypt(&encrypted, challenge.secret).unwrap();

        assert_eq!(challenge.answer, decrypted);
    }

    #[test]
    fn can_verify_correct_answer() {
        let secret = Secret::from("7LphV05vqV3oxYj831j97H3vs2g5wP89".to_string());
        let challenge = Base64Challenger::new(secret).expect("Creating challenge.");
        let encrypted = challenge.encrypt().unwrap();

        assert_ok!(Base64Challenger::verify(
            &encrypted,
            challenge.answer,
            challenge.secret
        ));
    }

    #[test]
    fn can_reject_incorrect_answer() {
        let secret = Secret::from("Zh20YpU56L5Ces0VffGl31rb2Km4k7Gr".to_string());
        let challenge = Base64Challenger::new(secret).expect("Creating challenge.");
        let encrypted = challenge.encrypt().unwrap();

        assert_err!(Base64Challenger::verify(
            &encrypted,
            String::from("badanswer"),
            challenge.secret
        ));
    }
}
