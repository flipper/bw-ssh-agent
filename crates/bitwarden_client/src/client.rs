pub use crate::models::{SyncResponse, Cipher, Profile, Organization};
use anyhow::Error;
use http::header;
use url::Url;

pub struct ClientSettings {
    pub api_url: String,
    pub access_token: String,
}

pub struct BitwardenClient {
    client: reqwest::Client,
    api_url: Url,
}

impl BitwardenClient {
    pub fn new(settings: ClientSettings) -> Result<Self, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Bitwarden-Client-Version",
            header::HeaderValue::from_static("2026.1.1"),
        );

        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(format!("Bearer {}", settings.access_token).as_str())?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let api_url = settings.api_url.parse::<Url>()?;

        Ok(Self { client, api_url })
    }

    pub async fn sync(&self) -> Result<SyncResponse, Error> {
        let url = self.api_url.join("sync")?;
        let response = self.client.get(url).send().await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                //let text = response.text().await?;
                //tokio::fs::write("/tmp/bitwarden_client.json", text.clone()).await?;
                let json = response.json::<SyncResponse>().await?;
                Ok(json)
            }
            Err(e) => {
                let text = response.text().await?;
                Err(Error::msg(format!("{}. Body: {}", e, text)))
            },
        }
    }
}
