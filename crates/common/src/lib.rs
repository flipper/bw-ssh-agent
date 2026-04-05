use anyhow::Error;
use bitwarden_client::{BitwardenClient, Cipher, Profile};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use bitwarden_client::utils::{decrypt_field, decrypt_field_as_string};
pub use secret_store::zbus_secret_store::ZBusSecretStore;
use secret_store::{DEFAULT_STORE, SecretStoreSendSync};
pub use secret_store::{SecretItem, SecretStore, SecretStoreInitializer};
pub use ssh_agent_lib;
use bitwarden_client::encryption::asymmetric::BWRsaPrivateKey;
use bitwarden_client::encryption::symmetric::Aes256CbcHmacKey;

pub const APP_ID: &str = "bw-ssh-agent";

pub mod agent;

const REFRESH_TOKEN_ENTRY: &str = "refresh-token";
const USER_KEY_ENC_ENTRY: &str = "user-key-enc";
const USER_KEY_MAC_ENTRY: &str = "user-key-mac";

pub async fn set_default_store(mut s: Arc<SecretStoreSendSync>) {
    if let Some(store) = Arc::get_mut(&mut s) {
        store.initialize().await.unwrap();
    }
    let mut default_store = DEFAULT_STORE.write().await;
    default_store.inner = Some(s);
}

pub async fn save_key(
    item: SecretItem,
    key: String,
    attrs: HashMap<&str, &str>,
) -> anyhow::Result<()> {
    match item.get().await? {
        Some(_) => Ok(()),
        None => {
            log::info!("Storing new key at: {}", item);
            item.set(key.as_bytes(), Some(attrs)).await?;
            Ok(())
        }
    }
}

pub async fn store_refresh_token(new_token: String) -> anyhow::Result<()> {
    let item = SecretItem::new(APP_ID.parse()?, REFRESH_TOKEN_ENTRY.parse()?);

    item.set(new_token.as_bytes(), None).await
}

pub async fn get_stored_refresh_token() -> anyhow::Result<Option<String>> {
    let item = SecretItem::new(APP_ID.parse()?, REFRESH_TOKEN_ENTRY.parse()?);

    let value = item.get().await?;

    match value {
        Some(value) => Ok(Some(String::from_utf8(value.to_vec())?)),
        None => Ok(None),
    }
}

pub async fn store_user_key(key: Aes256CbcHmacKey) -> Result<(), Error> {
    let user_key_enc = SecretItem::new(APP_ID.parse()?, USER_KEY_ENC_ENTRY.parse()?);
    let enc_key_slice = key.enc_key.as_slice();
    user_key_enc.set(enc_key_slice, None).await?;

    let mac_key_slice = key.mac_key.as_slice();
    let user_key_mac = SecretItem::new(APP_ID.parse()?, USER_KEY_MAC_ENTRY.parse()?);
    user_key_mac.set(mac_key_slice, None).await?;

    Ok(())
}

pub async fn get_stored_user_key() -> Result<Option<Aes256CbcHmacKey>, Error> {
    let user_key_enc = SecretItem::new(APP_ID.parse()?, USER_KEY_ENC_ENTRY.parse()?);
    let user_key_mac = SecretItem::new(APP_ID.parse()?, USER_KEY_MAC_ENTRY.parse()?);

    let user_key_enc = user_key_enc.get().await?;
    let user_key_mac = user_key_mac.get().await?;

    match (user_key_enc, user_key_mac) {
        (Some(enc), Some(mac)) => {
            let enc: [u8; 32] = enc.as_slice().try_into()?;
            let mac: [u8; 32] = mac.as_slice().try_into()?;

            Ok(Some(Aes256CbcHmacKey::new(enc, mac)))
        }
        _ => Ok(None),
    }
}

fn decrypt_key_from_user_encrypted_field(field: String, user_key: &Aes256CbcHmacKey) -> Result<Aes256CbcHmacKey, Error> {
    let decrypted = decrypt_field(field, user_key);
    match decrypted {
        Ok(v) => {
            let enc: [u8; 32] = v[0..32].try_into()?;
            let mac = v[32..64].try_into()?;
            Ok(Aes256CbcHmacKey::new(enc, mac))
        }
        Err(e) => {
            Err(anyhow::anyhow!("Failed to decrypt key: {}", e))
        }
    }
}

fn get_cipher_decryption_key(profile: &Profile, cipher: &Cipher, user_key: &Aes256CbcHmacKey) -> anyhow::Result<Option<Aes256CbcHmacKey>> {
    match &cipher.organization_id {
        Some(organization_id) => {
            let organization = profile.organizations.iter().find(|o| o.id == *organization_id);
            match organization {
                Some(organization) => {
                    let private_key = &profile.private_key;

                    let private_key_decrypted = decrypt_field(private_key.to_owned(), user_key)?;

                    let bw_rsa_key = BWRsaPrivateKey::new(private_key_decrypted)?;

                    let organization_key = decrypt_field(organization.key.clone(), &bw_rsa_key)?;

                    let enc_key: [u8; 32] = organization_key[0..32].try_into()?;
                    let mac_key: [u8; 32] = organization_key[32..64].try_into()?;

                    let key = Aes256CbcHmacKey::new(enc_key, mac_key);

                    Ok(Some(key))
                }
                None => {
                    Err(anyhow::anyhow!("Unable to find organization in the profile"))
                }
            }
        }
        None => match &cipher.key {
            Some(key) => {
                match decrypt_key_from_user_encrypted_field(key.clone(), &user_key) {
                    Ok(key) => Ok(Some(key)),
                    Err(e) => Err(e),
                }
            }
            None => Ok(None),
        }
    }
}

pub async fn sync_vault(bw_client: BitwardenClient, user_key: &Aes256CbcHmacKey) {
    let sync_response = bw_client.sync().await;

    if let Err(e) = sync_response {
        log::error!("Error syncing vault: {}", e);
        return;
    }

    let sync_response = sync_response.unwrap();

    let ssh_key_ciphers = sync_response.ciphers.into_iter().filter(|c| c.deleted_date.is_none()).filter(|c| c.ssh_key.is_some());

    let mut current_ids = HashSet::new();

    for cipher in ssh_key_ciphers {
        let decryption_key_owned = match get_cipher_decryption_key(&sync_response.profile, &cipher, &user_key) {
            Ok(decryption_key) => decryption_key,
            Err(e) => {
                log::error!("Error when getting decryption key for cipher: {}", e);
                continue;
            }
        };

        let decryption_key = decryption_key_owned.as_ref().unwrap_or(&user_key);

        let ssh_key = cipher.ssh_key.unwrap();

        let public_key_label = format!("pub-key:{}", cipher.id);
        let private_key_label = format!("pri-key:{}", cipher.id);

        let public_key_item =
            SecretItem::new(APP_ID.parse().unwrap(), public_key_label);
        let private_key_item =
            SecretItem::new(APP_ID.parse().unwrap(), private_key_label);

        let name = decrypt_field_as_string(cipher.name, decryption_key);

        if let Err(e) = name {
            log::error!("failed to decrypt name: {}. Cipher Id: {}", e, cipher.id);
            continue;
        }

        let name = name.unwrap();

        let fingerprint =
            decrypt_field_as_string(ssh_key.key_fingerprint, decryption_key);

        if let Err(e) = fingerprint {
            log::error!("failed to decrypt fingerprint: {:?}", e);
            continue;
        }

        let fingerprint = fingerprint.unwrap();

        let mut attributes = HashMap::<&str, &str>::new();
        attributes.insert("cipher-id", cipher.id.as_str());
        attributes.insert("fingerprint", fingerprint.as_str());
        attributes.insert("comment", name.as_str());

        let public_key = decrypt_field_as_string(ssh_key.public_key, decryption_key);
        if let Ok(public_key) = public_key {
            let mut attributes = attributes.clone();
            attributes.insert("type", "pub-key");
            match save_key(public_key_item, public_key, attributes).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("failed to save public key {:?}", e);
                }
            }
        }

        let private_key = decrypt_field_as_string(ssh_key.private_key, decryption_key);

        if let Ok(private_key) = private_key {
            let mut attributes = attributes.clone();
            attributes.insert("type", "pri-key");
            match save_key(private_key_item, private_key, attributes).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("failed to save private key {:?}", e);
                }
            }
        }

        current_ids.insert(cipher.id);
    }

    let store = DEFAULT_STORE.read().await;

    let mut filter = HashMap::<&str, &str>::new();
    filter.insert("type", "pub-key");

    let stored_ids = match store.search_items(APP_ID, filter).await {
        Ok(items) => {
            let mut stored_ids = HashSet::new();
            for item in items {
                let id = match item.get_attributes().await {
                    Ok(attrs) => {
                        match attrs {
                            None => {
                                log::error!("attributes not found: {}", item);
                                return;
                            }
                            Some(attrs) => {
                                let id = attrs.get("cipher-id").cloned();
                                id
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("Failed to get attributes for item: {}", e);
                        return;
                    }
                };

                if let Some(id) = id {
                    stored_ids.insert(id.to_owned());
                }
            }

            stored_ids
        },
        Err(e) => {
            log::error!("failed to search: {}", e);
            return
        }
    };

    for result in stored_ids.difference(&current_ids) {
        let mut filter = HashMap::<&str, &str>::new();
        filter.insert("cipher-id", result.as_str());

        let results = store.search_items(APP_ID, filter).await;

        match results {
            Ok(items) => {
                log::info!("Found {} items to remove", items.len());
                for item in items {
                    match item.delete().await {
                        Ok(_) => {
                            log::info!("Removed: {}", item);
                        }
                        Err(e) => {
                            log::error!("failed to delete item: {}", e);
                            continue;
                        }
                    };
                }
            },
            Err(e) => {
                log::error!("failed to search: {}", e);
                continue;
            }
        }
    }
}
