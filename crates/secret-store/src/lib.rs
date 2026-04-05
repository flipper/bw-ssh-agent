pub mod zbus_secret_store;

use anyhow::Error;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait::async_trait]
pub trait SecretStore {
    async fn get_item_attributes(&self, id: &str, key: &str) -> anyhow::Result<Option<HashMap<String, String>>>;

    async fn get_item_value(&self, id: &str, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
    async fn set_item_value(&self, id: &str, key: &str, value: Vec<u8>, attributes: Option<HashMap<&str, &str>>) -> anyhow::Result<()>;

    async fn delete_item(&self, id: &str, key: &str) -> anyhow::Result<()>;

    async fn search_items(&self, id: &str, attributes: HashMap<&str, &str>) -> anyhow::Result<Vec<SecretItem>>;
}

#[async_trait::async_trait]
pub trait SecretStoreInitializer: SecretStore {

    fn new() -> Self where Self: Sized;

    async fn initialize(&mut self) -> anyhow::Result<()>;
}

pub type SecretStoreSendSync = dyn SecretStoreInitializer + Send + Sync;

pub struct DefaultStore {
    pub inner: Option<Arc<SecretStoreSendSync>>,
}

#[async_trait::async_trait]
impl SecretStore for DefaultStore {
    async fn get_item_attributes(&self, id: &str, key: &str) -> anyhow::Result<Option<HashMap<String, String>>> {
        let inner = self.inner.as_ref().unwrap();
        inner.get_item_attributes(id, key).await
    }

    async fn get_item_value(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let inner = self.inner.as_ref().unwrap();
        inner.get_item_value(id, key).await
    }

    async fn set_item_value(&self, id: &str, key: &str, value: Vec<u8>, attributes: Option<HashMap<&str, &str>>) -> Result<(), Error> {
        let inner = self.inner.as_ref().unwrap();
        inner.set_item_value(id, key, value, attributes).await
    }

    async fn delete_item(&self, id: &str, key: &str) -> anyhow::Result<()> {
        let inner = self.inner.as_ref().unwrap();
        inner.delete_item(id, key).await
    }

    async fn search_items(&self, id: &str, attributes: HashMap<&str, &str>) -> anyhow::Result<Vec<SecretItem>> {
        let inner = self.inner.as_ref().unwrap();
        inner.search_items(id, attributes).await
    }
}

lazy_static! {
    pub static ref DEFAULT_STORE: RwLock<DefaultStore> = RwLock::new(DefaultStore { inner: None });
}

pub struct SecretItem {
    id: String,
    key: String,
}

impl Display for SecretItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretItem({}@{})", self.key, self.id)
    }
}

impl SecretItem {
    pub fn new(id: String, key: String) -> Self {
        SecretItem {
            id,
            key,
        }
    }

    pub async fn get_attributes(&self) -> anyhow::Result<Option<HashMap<String, String>>> {
        let store = DEFAULT_STORE.read().await;
        store.get_item_attributes(self.id.as_ref(), self.key.as_ref()).await
    }

    pub async fn get(&self) -> Result<Option<Vec<u8>>, Error> {
        let store = DEFAULT_STORE.read().await;
        store.get_item_value(self.id.as_ref(), &self.key).await
    }

    pub async fn set(&self, value: &[u8], attributes: Option<HashMap<&str, &str>>) -> Result<(), Error> {
        let store = DEFAULT_STORE.write().await;
        store.set_item_value(self.id.as_ref(), self.key.as_ref(), value.to_vec(), attributes).await
    }

    pub async fn delete(&self) -> anyhow::Result<()> {
        let store = DEFAULT_STORE.read().await;
        store.delete_item(self.id.as_ref(), self.key.as_ref()).await
    }
}