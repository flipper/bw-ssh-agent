
use crate::models::{
    ConnectTokenErrorResponse, ConnectTokenRequest, ConnectTokenSuccessResponse,
    MasterPasswordAuthenticationData, PreLoginRequest, PreLoginResponse, SendTwoFactorEmailRequest,
    TokenResponse,
};
use crate::utils::stretch_key;
use anyhow::Error;
use generic_array::GenericArray;
use http::{header, StatusCode};
use pbkdf2;
use reqwest::Response;
use serde::de::DeserializeOwned;
use sha2::Sha256;
use std::num::NonZeroU32;
use std::pin::Pin;
use thiserror::Error;
use url::Url;
use crate::encryption::EncryptedString;
use crate::encryption::symmetric::Aes256CbcHmacKey;

const PBKDF_SHA256_HMAC_OUT_SIZE: usize = 32;
const DEVICE_ID: &str = "d3511425-1904-44c6-a07a-96d9273fd5c3";
const DEVICE_TYPE: &str = "10";

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:149.0) Gecko/20100101 Firefox/149.0";

pub struct BitwardenAuthClient {
    client: reqwest::Client,
    identity_url: Url,
    api_url: Url,
}

#[derive(Debug, Clone, Default)]
pub struct PasswordLoginRequest {
    pub email: String,
    pub password: String,

    pub two_factor_token: Option<String>,
    pub two_factor_provider: Option<String>,
    pub two_factor_remember: Option<bool>,
    pub new_device_otp: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SendTwoFactorEmail {
    pub email: String,
    pub password: String,
}

pub struct KeyStore {
    pub refresh_token: Pin<String>,
    pub user_key: Aes256CbcHmacKey,
}

#[derive(Debug, Error)]
pub enum ClientError<O> {
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("http error ({0}): {1:?}")]
    Http(StatusCode, O),

    #[error("error: {0}")]
    Other(anyhow::Error),
}

trait ResponseDecode: Sized {
    type Output;
    async fn decode(r: Response) -> Result<Self::Output, reqwest::Error>;
}

impl ResponseDecode for () {
    type Output = ();

    async fn decode(_: Response) -> Result<Self, reqwest::Error> {
        Ok(())
    }
}

pub struct Json<T>(T);

impl<T> ResponseDecode for Json<T>
where
    T: DeserializeOwned,
{
    type Output = T;

    async fn decode(r: reqwest::Response) -> Result<Self::Output, reqwest::Error> {
        let v = r.json::<T>().await?;
        Ok(v)
    }
}
impl BitwardenAuthClient {
    async fn handle_response<T, O>(r: Response) -> Result<T::Output, ClientError<O::Output>>
    where
        T: ResponseDecode,
        O: ResponseDecode,
    {
        match r.error_for_status_ref() {
            Ok(_) => {
                let output = T::decode(r).await.map_err(ClientError::Request)?;
                Ok(output)
            }
            Err(e) => {
                let output = O::decode(r).await.map_err(ClientError::Request)?;
                Err(ClientError::Http(e.status().unwrap(), output))
            }
        }
    }

    fn get_cleaned_email(email: String) -> String {
        email.trim().to_lowercase()
    }

    pub fn new(identity_url: String, api_url: String) -> Result<Self, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Bitwarden-Client-Version",
            header::HeaderValue::from_static("2026.3.1"),
        );
        headers.insert(
            "Bitwarden-Client-Name",
            header::HeaderValue::from_static("web"),
        );
        headers.insert("device-type", header::HeaderValue::from_static(DEVICE_TYPE));
        headers.insert(
            "device-identifier",
            header::HeaderValue::from_static(DEVICE_ID),
        );
        headers.insert("User-Agent", header::HeaderValue::from_static(USER_AGENT));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let identity_url = identity_url.parse::<Url>()?;
        let api_url = api_url.parse::<Url>()?;

        Ok(Self {
            client,
            identity_url,
            api_url,
        })
    }

    pub async fn pre_login(&self, request: PreLoginRequest) -> Result<PreLoginResponse, Error> {
        let url = self.identity_url.join("accounts/prelogin/password")?;

        let r = self.client.post(url).json(&request).send().await?;

        match r.error_for_status() {
            Ok(res) => {
                let json = res.json::<PreLoginResponse>().await?;
                log::info!("Pre-login response: {:?}", json);
                Ok(json)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn get_master_password_auth_data(
        email: String,
        password: &[u8],
        kdf_iterations: NonZeroU32,
    ) -> MasterPasswordAuthenticationData {
        let salt = email.as_bytes();

        let master_key = pbkdf2::pbkdf2_hmac_array::<Sha256, PBKDF_SHA256_HMAC_OUT_SIZE>(
            password,
            salt,
            u32::from(kdf_iterations),
        );

        // 1 = Server Auth
        let master_key_hash = pbkdf2::pbkdf2_hmac_array::<Sha256, PBKDF_SHA256_HMAC_OUT_SIZE>(
            master_key.as_slice(),
            password,
            1,
        );

        MasterPasswordAuthenticationData {
            //kdf: kdf_iterations,
            salt: email,
            master_password_authentication_hash: master_key_hash.to_vec(),
            master_key: master_key.to_vec(),
        }
    }

    pub async fn send_two_factor_email(
        &self,
        request: SendTwoFactorEmail,
    ) -> Result<(), ClientError<()>> {
        let cleaned_email = Self::get_cleaned_email(request.email);
        let password_buf = request.password.as_bytes();

        let pre_login = self
            .pre_login(PreLoginRequest {
                email: cleaned_email.clone(),
            })
            .await
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let master_auth_data = Self::get_master_password_auth_data(
            cleaned_email.clone(),
            password_buf,
            pre_login.kdf_iterations,
        );

        let url = self
            .api_url
            .join("two-factor/send-email-login")
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let json_request = SendTwoFactorEmailRequest {
            email: cleaned_email,
            master_password_hash: master_auth_data.master_password_authentication_hash,
            device_identifier: DEVICE_ID.to_string(),
            auth_request_id: Some("".to_string()),
            auth_request_access_code: Some("".to_string()),
            sso_email2_fa_session_token: Some("".to_string()),

            otp: None,
            secret: None,
        };

        let response = self.client.post(url).json(&json_request).send().await?;

        Self::handle_response::<(), ()>(response).await
    }

    pub async fn login_password(
        &self,
        request: PasswordLoginRequest,
    ) -> Result<KeyStore, ClientError<ConnectTokenErrorResponse>> {
        let cleaned_email = Self::get_cleaned_email(request.email);
        let password_buf = request.password.as_bytes();

        let pre_login = self
            .pre_login(PreLoginRequest {
                email: cleaned_email.clone(),
            })
            .await
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let master_auth_data = Self::get_master_password_auth_data(
            cleaned_email,
            password_buf,
            pre_login.kdf_iterations,
        );

        let password_token_request = ConnectTokenRequest {
            grant_type: "password".to_string(),
            scope: Some("api offline_access".to_string()),
            client_id: "web".to_string(),
            device_type: Some(DEVICE_TYPE.to_string()),
            device_identifier: Some(DEVICE_ID.to_string()),
            device_name: Some("firefox".to_string()),
            master_password_hash: Some(master_auth_data.master_password_authentication_hash),
            email: Some(master_auth_data.salt),
            two_factor_token: request.two_factor_token,
            two_factor_provider: request.two_factor_provider,
            two_factor_remember: request.two_factor_remember,
            new_device_otp: request.new_device_otp,
            ..Default::default()
        };

        log::info!(
            "ConnectTokenRequest: {}",
            serde_json::to_string_pretty(&password_token_request)
                .map_err(|e| ClientError::Other(Error::from(e)))?
        );

        let connect_token_url = self
            .identity_url
            .join("connect/token")
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let connect_token_response = self
            .client
            .post(connect_token_url)
            .header(header::ACCEPT, "application/json")
            .form(&password_token_request)
            .send()
            .await?;

        let response = Self::handle_response::<
            Json<ConnectTokenSuccessResponse>,
            Json<ConnectTokenErrorResponse>,
        >(connect_token_response)
        .await?;

        let master_key: [u8; 32] = master_auth_data
            .master_key
            .as_slice()
            .try_into()
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let pinned_master_key = Pin::new(Box::new(GenericArray::from(master_key)));

        let stretched_master_key = stretch_key(&pinned_master_key);

        let encrypted_user_key = response
            .user_decryption_options
            .master_password_unlock
            .master_key_encrypted_user_key;

        let encrypted_user_key = EncryptedString::try_from(encrypted_user_key)
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let user_key = encrypted_user_key
            .decrypt(&stretched_master_key)
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let user_key_enc: [u8; 32] = user_key[0..32]
            .try_into()
            .map_err(|e| ClientError::Other(Error::from(e)))?;
        let user_key_hmac: [u8; 32] = user_key[32..64]
            .try_into()
            .map_err(|e| ClientError::Other(Error::from(e)))?;

        let stretched_user_key = Aes256CbcHmacKey::new(user_key_enc, user_key_hmac);

        let refresh_token = Pin::new(response.refresh_token);

        Ok(KeyStore {
            refresh_token,
            user_key: stretched_user_key,
        })
    }

    pub async fn renew_token(&self, refresh_token: String) -> Result<TokenResponse, Error> {
        let request = ConnectTokenRequest {
            grant_type: "refresh_token".to_string(),
            refresh_token: Some(refresh_token),
            client_id: "web".to_string(),
            ..Default::default()
        };

        let url = self.identity_url.join("connect/token")?;

        let response = self.client.post(url).form(&request).send().await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let json = response.json::<TokenResponse>().await?;
                Ok(json)
            }
            Err(e) => {
                let text = response.text().await?;
                Err(Error::msg(format!("{}. Body: {}", e, text)))
            }
        }
    }
}
