use base64::Engine;
use base64::engine::general_purpose::STANDARD;

#[derive(Debug)]
pub struct Base64ImageUrl(String);

impl AsRef<str> for Base64ImageUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Base64ImageUrl {
    pub fn parse(s: String) -> Result<Base64ImageUrl, String> {
        let segments: Vec<&str> = s.split(",").collect();

        if segments.len() != 2 || !segments[0].contains("data:") {
            return Err(String::from("Invalid base64 data URI."));
        } else if !segments[0].contains(";base64") {
            return Err(String::from("Data URI is missing base64 extension."));
        }

        let data = segments[1];
        let segments: Vec<&str> = segments[0].split(";").collect();
        let segments: Vec<&str> = segments[0].split(":").collect();
        let media_type = segments[1];

        if ![
            "image/avif",
            "image/bmp",
            "image/gif",
            "image/jpeg",
            "image/png",
            "image/svg+xml",
            "image/tiff",
            "image/webp",
        ]
        .contains(&media_type)
        {
            return Err(String::from("Invalid image media type."));
        };

        match STANDARD.decode(data) {
            Err(error) => Err(error.to_string()),
            Ok(_) => Ok(Self(s)),
        }
    }

    pub fn validate_size_limit(self, size: usize) -> Result<Base64ImageUrl, String> {
        let segments: Vec<&str> = self.as_ref().split(',').collect();
        let total_bytes: usize = (segments[1].len() * 3) / 4;

        if total_bytes > size {
            Err(format!(
                "File size: {:?}kb exceeds size limit: {:?}kb.",
                total_bytes / 1024,
                size / 1024
            ))
        } else {
            Ok(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::Base64ImageUrl;
    use claims::{assert_err, assert_ok};

    #[test]
    fn whitespace_only_base64_urls_are_rejected() {
        let base64_url = " ".to_string();

        assert_err!(Base64ImageUrl::parse(base64_url));
    }

    #[test]
    fn empty_base64_url_is_rejected() {
        let base64_url = "".to_string();

        assert_err!(Base64ImageUrl::parse(base64_url));
    }

    #[test]
    fn dataless_base64_url_is_rejected() {
        let base64_url = "data:video/webm;base64,".to_string();

        assert_err!(Base64ImageUrl::parse(base64_url));
    }

    #[test]
    fn data_only_base64_url_is_rejected() {
        let base64_url = "iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==".to_string();

        assert_err!(Base64ImageUrl::parse(base64_url));
    }

    #[test]
    fn video_mime_type_base64_url_is_rejected() {
        let base64_url = "data:video/webm;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==".to_string();

        assert_err!(Base64ImageUrl::parse(base64_url));
    }

    #[test]
    fn a_valid_image_base64_url_is_parsed_successfully() {
        let base64_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==".to_string();

        assert_ok!(Base64ImageUrl::parse(base64_url));
    }

    #[test]
    fn a_valid_image_base64_url_validates_its_length() {
        let base64_url = "data:image/svg+xml;base64,PHN2ZyBoZWlnaHQ9IjgwMCIgd2lkdGg9IjgwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB2aWV3Qm94PSIwIDAgNTAzLjA2OCA1MDMuMDY4Ij4KICA8cGF0aAogICAgZD0iTTc4Ljg3NiAzNTMuMjA4YTcuNSA3LjUgMCAwMC0xMS43MzUtMS40NDRsLTIyLjYxIDIyLjYxYTIzLjQ2NyAyMy40NjcgMCAwMC0zLjUzNCAyOC43MDhsMzMuOTQxIDU2LjU2OGM1LjEzMiA4LjU1NSAxMy4wMjkgMTAuNzEgMTcuMjggMTEuMjMzLjg0Ni4xMDQgMS44NDcuMTgxIDIuOTY0LjE4MSA0LjQ5OCAwIDEwLjg3My0xLjIzOCAxNi41MjMtNi44ODhsMTYuOTU0LTE2Ljk1NGE3LjUgNy41IDAgMDAxLjEyOC05LjE2MmwtNTAuOTExLTg0Ljg1MnptMjIuMjIzIDEwMC4zNjJjLTIuNDI0IDIuNDI0LTUuMTcgMi42NTctNy4wNDggMi40MjYtMS44NzgtLjIzMS00LjQ4Ny0xLjEyNC02LjI1LTQuMDYzTDUzLjg2IDM5NS4zNjRhOC40ODkgOC40ODkgMCAwMTEuMjc4LTEwLjM4NGwxNS44MTgtMTUuODE4IDQyLjk1NyA3MS41OTUtMTIuODE0IDEyLjgxM3oiIC8+CiAgPHBhdGgKICAgIGQ9Ik00OTMuMTA1IDc5LjM3OWwtMTguMzgxLTguNTU3YTcxLjE1MSA3MS4xNTEgMCAwMC0zLjA2NC04LjM0MWg5LjcxMWE3LjUgNy41IDAgMDAwLTE1aC0xOC42MTNhNzEuNjk0IDcxLjY5NCAwIDAwLTYtNi44NDQgNzIuNjY4IDcyLjY2OCAwIDAwLTIuODE4LTIuNjY1bDEwLjIzOC02LjYzMWE3LjUgNy41IDAgMDAtOC4xNTQtMTIuNTlsLTE1LjI0NyA5Ljg3NWE3MC45ODYgNzAuOTg2IDAgMDAtMTIuNzI1LTUuNDhsMy40MjgtMy40MjhhNy41IDcuNSAwIDAwMC0xMC42MDYgNy41IDcuNSAwIDAwLTEwLjYwNiAwbC0xMC43MzUgMTAuNzM0YTcyLjA4NyA3Mi4wODcgMCAwMC0xMC4wMjEuMTU0bDMuOTU2LTkuNjU0YTcuNSA3LjUgMCAxMC0xMy44OC01LjY4OEwzODIuMzQ5IDIzLjhhNzEuMDAzIDcxLjAwMyAwIDAwLTguMjQ4IDMuNTIxdi05LjY2NWE3LjUgNy41IDAgMDAtMTUgMHYxOS43NjZhNzIuNDMxIDcyLjQzMSAwIDAwLTMuNDYgMy4yMTVsLTQuMTY3IDQuMTY3di00LjUyMWMwLTQuMTQzLTMuMzU4LTcuNS03LjUtNy41cy03LjUgMy4zNTctNy41IDcuNXYxOS41MjFsLTcuNjI3IDcuNjI4VjYyLjkxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNy42MjcgNy42Mjd2LTQuNTIxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNy42MjcgNy42Mjh2LTQuNTIxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNy42MjcgNy42Mjd2LTQuNTIxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNy42MjcgNy42Mjh2LTQuNTIxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNy42MjcgNy42Mjd2LTQuNTIxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNy42MjcgNy42Mjh2LTQuNTIxYTcuNSA3LjUgMCAwMC0xNSAwdjE5LjUyMWwtNi4wMDYgNi4wMDYtMi4xMDktNS41NmMtLjAwNy0uMDItLjAxNy0uMDM4LS4wMjUtLjA1N2E3LjE0MiA3LjE0MiAwIDAwLS4yMDQtLjQ2NmMtLjAzNS0uMDc2LS4wNjgtLjE1My0uMTA1LS4yMjctLjA2NS0uMTI3LS4xMzgtLjI1LS4yMTEtLjM3NC0uMDU3LS4wOTctLjExMS0uMTk3LS4xNzItLjI5MS0uMDU2LS4wODYtLjExOC0uMTY4LS4xNzgtLjI1Mi0uMDg4LS4xMjUtLjE3Ni0uMjUxLS4yNzItLjM3LS4wNDYtLjA1Ny0uMDk3LS4xMTEtLjE0NS0uMTY3YTcuMjE0IDcuMjE0IDAgMDAtLjM2MS0uMzk5bC0uMDM3LS4wNDFjLS4wNDEtLjA0MS0uMDg1LS4wNzUtLjEyNy0uMTE1YTguMTQzIDguMTQzIDAgMDAtLjM3OS0uMzQzYy0uMDc4LS4wNjQtLjE1OC0uMTI0LS4yMzgtLjE4Ni0uMTEtLjA4NC0uMjItLjE2OC0uMzM0LS4yNDZhNy4wMDggNy4wMDggMCAwMC0uMzE2LS4yMDEgNi41NjcgNi41NjcgMCAwMC0uNjYtLjM1OSA3LjgwMiA3LjgwMiAwIDAwLS4yNzktLjEzIDYuOTQyIDYuOTQyIDAgMDAtLjM4NC0uMTUxIDYuOSA2LjkgMCAwMC0uNjY4LS4yMTEgNy4wNTMgNy4wNTMgMCAwMC0xLjM3OC0uMjMgNi44MzcgNi44MzcgMCAwMC0uNDMtLjAyMWMtLjA5Ny0uMDAxLS4xOTQuMDAxLS4yOTEuMDAzYTcuNzA5IDcuNzA5IDAgMDAtLjQyOC4wMjEgNyA3IDAgMDAtLjMyMS4wMzYgNi40MDcgNi40MDcgMCAwMC0uMzg2LjA1NmMtLjEzLjAyMy0uMjU5LjA1My0uMzg4LjA4My0uMTA0LjAyNC0uMjA3LjA0Ny0uMzExLjA3NmE3Ljk1OCA3Ljk1OCAwIDAwLS40NjguMTUxYy0uMDU5LjAyMS0uMTE5LjAzNS0uMTc3LjA1OC0uMDE5LjAwNy0uMDM2LjAxNy0uMDU1LjAyMy0uMTYuMDYzLS4zMTcuMTM0LS40NzQuMjA4LS4wNzMuMDM0LS4xNDguMDY1LS4yMTkuMTAyLS4xMy4wNjYtLjI1NS4xNDEtLjM4MS4yMTUtLjA5NS4wNTYtLjE5MS4xMDgtLjI4My4xNjctLjA4OS4wNTgtLjE3NC4xMjMtLjI2MS4xODQtLjEyMi4wODYtLjI0NC4xNzEtLjM1OS4yNjQtLjA2MS4wNDktLjExOS4xMDQtLjE3OS4xNTVhNy42MjUgNy42MjUgMCAwMC0uMzg4LjM1MWwtLjA0My4wMzktOC4zODkgOC4zOS00Ljk0Ny0xNC44MzljLTIuNTk0LTcuNzgzLTguOTEtMTMuNTU1LTE2Ljg5NS0xNS40NC03Ljk4My0xLjg4NC0xNi4yMTUuNDUzLTIyLjAxNiA2LjI1NEw0NC45MiAyNjAuODVjLTguOTcyIDguOTcyLTEzLjkxMyAyMC45LTEzLjkxMyAzMy41ODh2NDguMjA3bC05LjEgOS4xMDFhNTUuNDIgNTUuNDIgMCAwMC04LjM0NyA2Ny43OTlsMzMuOTQxIDU2LjU2OGM4Ljg1NCAxNC43NTcgMjMuNzI5IDI0LjQyNyA0MC44MSAyNi41MjkgMi4zMS4yODUgNC42MTIuNDI1IDYuOS40MjUgMTQuNjMxIDAgMjguNjAyLTUuNzQxIDM5LjEyNS0xNi4yNjVsODQuODM2LTg0LjgzNmM0LjI5LTQuMjkgNy4xOTUtOS43MDkgOC41NjMtMTUuNDUyYTcuNDU3IDcuNDU3IDAgMDAzLjEwNC42NzkgNy41IDcuNSAwIDAwNy41LTcuNXYtMTkuNTIxbDcuNjI3LTcuNjI3djQuNTIxYzAgNC4xNDMgMy4zNTggNy41IDcuNSA3LjVzNy41LTMuMzU3IDcuNS03LjV2LTE5LjUyMWw3LjYyNy03LjYyN3Y0LjUyMWE3LjUgNy41IDAgMDAxNSAwdi0xOS41MjFsNy42MjctNy42Mjd2NC41MjFjMCA0LjE0MyAzLjM1OCA3LjUgNy41IDcuNXM3LjUtMy4zNTcgNy41LTcuNXYtMTkuNTIxbDcuNjI3LTcuNjI3djQuNTIxYTcuNSA3LjUgMCAwMDE1IDB2LTE5LjUyMWw3LjYyNy03LjYyN3Y0LjUyMWE3LjUgNy41IDAgMDAxNSAwdi0xOS41MjFsNy42MjctNy42Mjd2NC41MjFhNy41IDcuNSAwIDAwMTUgMFYyMjQuNDFsNy42MjctNy42Mjd2NC41MjFhNy41IDcuNSAwIDAwMTUgMHYtMTkuNTIxbDcuNjI3LTcuNjI3djQuNTIxYTcuNSA3LjUgMCAwMDE1IDB2LTE5LjUyMWw3LjYyNy03LjYyN3Y0LjUyMWE3LjUgNy41IDAgMDAxNSAwdi0xOS41MjFsNy42MjctNy42Mjh2NC41MjFhNy41IDcuNSAwIDAwMTUgMHYtMjEuMDE1YTcwLjgyMSA3MC44MjEgMCAwMDcuMTgyLTEyLjgwNWw0Ljg3OCA3LjIyNGE3LjQ5MSA3LjQ5MSAwIDAwNi4yMjIgMy4zMDMgNy41IDcuNSAwIDAwNi4yMDgtMTEuNjk4bC0xMi4wNjQtMTcuODY1Yy41MTYtMy45MzIuNzA1LTcuODk3LjU2OC0xMS44NTVsOS4xNjkgNC4yNjhhNy41IDcuNSAwIDAwOS45NjQtMy42MzQgNy40OTYgNy40OTYgMCAwMC0zLjYzMS05Ljk2NnpNNDYuMDA0IDI5NC40NGMwLTguNjgxIDMuMzgxLTE2Ljg0MyA5LjUxOS0yMi45ODFsNjAuNDg4LTYwLjQ4N2MyLjEzLTIuMTMgNS4wMzQtMi45NTQgNy45NjMtMi4yNjMgMi45MzEuNjkyIDUuMTU5IDIuNzI4IDYuMTExIDUuNTg1bDcuMzE4IDIxLjk1NC05MS4zOTkgOTEuMzk5VjI5NC40NHptNzcuNzIzIDE4MS43NTdjLTguODggOC44OC0yMS4xMiAxMy4wOTItMzMuNTg2IDExLjU1OS0xMi40NjQtMS41MzUtMjMuMzE5LTguNTkxLTI5Ljc4LTE5LjM2TDI2LjQyIDQxMS44MjhhNDAuNDQyIDQwLjQ0MiAwIDAxNi4wOTEtNDkuNDc1bDEyNy41NDUtMTI3LjU0NkwyMTIuNjY0IDM3My41YzIuMjYzIDUuOTY3LjYxNSAxMy4xNDUtNC4xMDEgMTcuODZsLTg0LjgzNiA4NC44Mzd6bTE5Mi4zMTYtMjE0Ljk0M2wtMjIuNjI3IDIyLjYyNy0uMDAxLjAwMS0yMi42MjQgMjIuNjI0LS4wMDcuMDA2LTIyLjYyMSAyMi42MjEtLjAwNy4wMDctMjIuNjIgMjIuNjItLjAwOC4wMDctMy42NzMgMy42NzMtNDQuMDc1LTExNi4xOTdhNy40NzUgNy40NzUgMCAwMDEuNzktMS4zMjRMMzQ5LjI3NiA2OC4yMTVsLjAwMi0uMDAyIDE2Ljk2OS0xNi45NjljMjIuMDMtMjIuMDMgNTcuODc1LTIyLjAyOSA3OS45MDMgMGE1Ni42MjIgNTYuNjIyIDAgMDE1LjY2MyA2LjYwNiA3LjUwNCA3LjUwNCAwIDAwMS43NTUgMi41NTJjMTQuMjM3IDIxLjg5MiAxMS43NjkgNTEuNTU3LTcuNDE4IDcwLjc0NUwzMTYuMDQzIDI2MS4yNTR6IiAvPgogIDxwYXRoCiAgICBkPSJNNDA2LjE5OSA1MS42OTRjLTEwLjU1MSAwLTIwLjQ3IDQuMTA5LTI3LjkzIDExLjU2OS03LjQ2MSA3LjQ2MS0xMS41NyAxNy4zOC0xMS41NyAyNy45MzFzNC4xMDkgMjAuNDcxIDExLjU3IDI3LjkzMWM3LjQ2IDcuNDYxIDE3LjM3OSAxMS41NjkgMjcuOTMgMTEuNTY5IDEwLjU1MSAwIDIwLjQ3LTQuMTA4IDI3LjkzMS0xMS41NjkgNy40NjEtNy40NiAxMS41NjktMTcuMzggMTEuNTY5LTI3LjkzMXMtNC4xMDgtMjAuNDctMTEuNTY5LTI3LjkzMWMtNy40NjEtNy40Ni0xNy4zOC0xMS41NjktMjcuOTMxLTExLjU2OXptMTcuMzI0IDU2LjgyNWMtNC42MjcgNC42MjgtMTAuNzggNy4xNzYtMTcuMzI0IDcuMTc2YTI0LjMzOCAyNC4zMzggMCAwMS0xNy4zMjQtNy4xNzYgMjQuMzQgMjQuMzQgMCAwMS03LjE3Ni0xNy4zMjRjMC02LjU0NCAyLjU0OS0xMi42OTYgNy4xNzYtMTcuMzI0YTI0LjM0IDI0LjM0IDAgMDExNy4zMjQtNy4xNzYgMjQuMzQgMjQuMzQgMCAwMTE3LjMyNCA3LjE3NiAyNC4zNCAyNC4zNCAwIDAxNy4xNzYgMTcuMzI0IDI0LjMzOCAyNC4zMzggMCAwMS03LjE3NiAxNy4zMjR6TTEwNi4zODYgMzMwLjYyNmE3LjQ3NCA3LjQ3NCAwIDAwNS4zMDMtMi4xOTdsMzMuOTQxLTMzLjk0MWE3LjUgNy41IDAgMDAwLTEwLjYwNiA3LjUgNy41IDAgMDAtMTAuNjA2IDBsLTMzLjk0MSAzMy45NDFhNy41IDcuNSAwIDAwMCAxMC42MDYgNy40NzQgNy40NzQgMCAwMDUuMzAzIDIuMTk3ek0xMTIuMzk2IDM0MC40NWE3LjUgNy41IDAgMDA1LjMwMyAxMi44MDMgNy40NzQgNy40NzQgMCAwMDUuMzAzLTIuMTk3bDMzLjk0MS0zMy45NDFhNy41IDcuNSAwIDAwMC0xMC42MDYgNy40OTggNy40OTggMCAwMC0xMC42MDYgMGwtMzMuOTQxIDMzLjk0MXpNMTU3LjY1MSAzMjkuMTM2bC0zMy45NDEgMzMuOTQxYTcuNSA3LjUgMCAwMDAgMTAuNjA2YzEuNDY0IDEuNDY1IDMuMzg0IDIuMTk3IDUuMzAzIDIuMTk3czMuODM5LS43MzIgNS4zMDMtMi4xOTdsMzMuOTQxLTMzLjk0MWE3LjUgNy41IDAgMDAwLTEwLjYwNiA3LjUgNy41IDAgMDAtMTAuNjA2IDB6IiAvPgo8L3N2Zz4=".to_string();

        assert_err!(
            Base64ImageUrl::parse(base64_url)
                .unwrap()
                .validate_size_limit(1024 * 2)
        );
    }
}
