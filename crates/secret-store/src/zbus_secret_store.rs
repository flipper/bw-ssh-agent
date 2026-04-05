use crate::{SecretItem, SecretStore, SecretStoreInitializer};
use async_trait::async_trait;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use futures::future::join_all;
use secret_service::{EncryptionType, Item, SecretService};
use std::collections::HashMap;

pub struct ZBusSecretStore {
    ss: Option<SecretService<'static>>,
}

#[async_trait]
impl SecretStoreInitializer for ZBusSecretStore {
    fn new() -> Self {
        Self { ss: None }
    }

    async fn initialize(&mut self) -> anyhow::Result<()> {
        loop {
            let ss = SecretService::connect(EncryptionType::Dh).await;

            if let Err(e) = &ss {
                log::error!("error connecting to secret-store: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            let ss = ss?;

            let default_collection = match ss.get_default_collection().await {
                Ok(c) => {
                    c
                }
                Err(e) => {
                    log::error!("error while getting the default collection: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue
                }
            };

            log::info!("Found default collection: {}. Checking if it is unlocked...", default_collection.collection_path);

            match default_collection.unlock().await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("error while unlocking the secret-store: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            }

            self.ss = Some(ss);
            log::info!("zbus secret store initialized");

            break;
        }

        Ok(())
    }
}

fn to_label(id: &str, key: &str) -> String {
    format!("{}@{}", key, id)
}

fn from_label(label: &str) -> SecretItem {
    let split: Vec<&str> = label.split('@').collect();
    if split.len() != 2 {
        panic!("Invalid label: {}", label);
    }

    SecretItem::new(split[1].to_string(), split[0].to_string())
}

impl ZBusSecretStore {
    async fn get_item(&'_ self, id: &str, key: &str) -> anyhow::Result<Option<Item<'_>>> {
        let ss = self.ss.as_ref().unwrap();

        let collection = ss.get_default_collection().await?;

        let mut attrs = HashMap::new();
        attrs.insert("service", id);
        attrs.insert("username", key);

        let items = collection.search_items(attrs).await?;

        if let Some(item) = items.first() {
            let item_path = item.item_path.clone();
            let item = ss.get_item_by_path(item_path).await?;
            item.ensure_unlocked().await?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl SecretStore for ZBusSecretStore {
    async fn get_item_attributes(&self, id: &str, key: &str) -> anyhow::Result<Option<HashMap<String, String>>> {
        let item = self.get_item(id, key).await?;

        match item {
            None => Ok(None),
            Some(i) => {
                let value = i.get_attributes().await?;
                Ok(Some(value))
            }
        }
    }

    async fn get_item_value(&self, id: &str, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let item = self.get_item(id, key).await?;

        match item {
            None => Ok(None),
            Some(i) => {
                let value = i.get_secret().await?;
                let value = BASE64_STANDARD.decode(value)?;

                Ok(Some(value))
            }
        }
    }

    async fn set_item_value(
        &self,
        id: &str,
        key: &str,
        value: Vec<u8>,
        attributes: Option<HashMap<&str, &str>>,
    ) -> anyhow::Result<()> {
        let label = to_label(id, key);

        let ss = self.ss.as_ref().unwrap();

        let collection = ss.get_default_collection().await?;


        let mut attributes = attributes.unwrap_or_default();
        attributes.insert("service", id);
        attributes.insert("username", key);

        let encoded_value = BASE64_STANDARD.encode(value);

        let encoded_value = encoded_value.as_bytes();

        let _ = collection
            .create_item(
                label.as_str(),
                attributes,
                encoded_value,
                true,
                "text/plain",
            )
            .await?;

        Ok(())
    }

    async fn delete_item(&self, id: &str, key: &str) -> anyhow::Result<()> {
        let item = self.get_item(id, key).await?;
        match item {
            None => Err(anyhow::anyhow!("Item not found")),
            Some(i) => {
                i.delete().await?;
                Ok(())
            }
        }
    }

    async fn search_items(
        &self,
        id: &str,
        mut attributes: HashMap<&str, &str>,
    ) -> anyhow::Result<Vec<SecretItem>> {
        let ss = self.ss.as_ref().unwrap();

        attributes.insert("service", id);

        let results = ss.search_items(attributes).await?;

        let results = join_all(results.unlocked.iter().map(|i| i.get_label())).await;

        let results = results
            .into_iter()
            .filter_map(|res| res.ok())
            .map(|i| from_label(i.as_str()))
            .collect::<Vec<_>>();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn simple_test() {
        let mut store = ZBusSecretStore::new();

        store
            .initialize()
            .await
            .expect("failed to initialize store");

        let buf = "hello".as_bytes();
        store
            .set_item_value("id", "key", Vec::from(buf), None)
            .await
            .expect("unable to set value");

        let stored_buf = store
            .get_item_value("id", "key")
            .await
            .expect("unable to get value");

        match stored_buf {
            None => {
                assert!(false);
            }
            Some(stored_buf) => {
                assert_eq!(buf, stored_buf);
            }
        }
    }
}
