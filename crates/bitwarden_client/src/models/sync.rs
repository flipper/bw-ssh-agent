use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginItem {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SSHKeyItem {
    pub key_fingerprint: String,
    pub private_key: String,
    pub public_key: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Cipher {
    #[serde(rename = "type")]
    pub r#type: i32,

    pub id: String,
    pub name: String,

    #[serde(default)]
    pub login: Option<LoginItem>,
    #[serde(default)]
    pub ssh_key: Option<SSHKeyItem>,

    pub deleted_date: Option<String>,

    /// Item Encryption Key
    pub key: Option<String>,

    pub organization_id: Option<String>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub id: String,
    pub key: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub private_key: String,
    pub organizations: Vec<Organization>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub profile: Profile,
    pub ciphers: Vec<Cipher>,
}