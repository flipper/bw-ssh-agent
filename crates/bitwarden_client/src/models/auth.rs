use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::base64::Standard;
use serde_with::formats::Padded;
use serde_with::serde_as;
use std::num::NonZeroU32;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreLoginRequest {
    pub email: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreLoginResponse {
    pub kdf: u32,
    pub kdf_iterations: NonZeroU32,
}

#[derive(Debug)]
pub struct MasterPasswordAuthenticationData {
    //pub kdf: NonZeroU32,
    pub salt: String,
    pub master_password_authentication_hash: Vec<u8>,
    pub master_key: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectTokenRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(rename = "client_id")]
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,

    #[serde(rename = "grant_type")]
    pub grant_type: String,

    #[serde(rename = "username")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde_as(as = "Option<Base64<Standard, Padded>>")]
    #[serde(rename = "password")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_password_hash: Option<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "refresh_token")]
    pub refresh_token: Option<String>,

    #[serde(rename = "twoFactorToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_token: Option<String>,
    #[serde(rename = "twoFactorProvider")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_provider: Option<String>,
    #[serde(rename = "twoFactorRemember")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_remember: Option<bool>,

    #[serde(rename = "newDeviceOtp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_device_otp: Option<String>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Kdf {
    #[allow(unused)]
    pub iterations: NonZeroU32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MasterPasswordUnlock {
    #[allow(unused)]
    pub kdf: Kdf,
    pub master_key_encrypted_user_key: String,
    #[allow(unused)]
    pub salt: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct UserDecryptionOptions {
    pub master_password_unlock: MasterPasswordUnlock,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ConnectTokenSuccessResponse {
    #[allow(unused)]
    pub private_key: String,

    pub user_decryption_options: UserDecryptionOptions,

    #[serde(rename = "access_token")]
    #[allow(unused)]
    pub access_token: String,
    #[serde(rename = "refresh_token")]
    pub refresh_token: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ConnectTokenErrorResponse {
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "error_description")]
    pub error_description: String,

    pub two_factor_providers: Option<Vec<String>>,
    pub sso_email_2fa_session_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}


#[serde_as]
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SendTwoFactorEmailRequest {
    pub email: String,
    pub device_identifier: String,
    #[serde_as(as = "Base64<Standard, Padded>")]
    pub master_password_hash: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_request_access_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sso_email2_fa_session_token: Option<String>,
}
