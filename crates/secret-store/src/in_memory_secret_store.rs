/*use crate::secret_store::SecretStoreInitializer;
use crate::{SecretItem, SecretStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct InMemorySecretStore {
    pub data: Arc<Mutex<Option<HashMap<String, Vec<u8>>>>>,
}

#[async_trait]
impl SecretStore for InMemorySecretStore {
    async fn get_item_value(&self, id: &str, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let storage_key = format!("{}:{}", id, key);
        let data = self.data.lock().await;
        let data = data.as_ref().unwrap();

        Ok(data.get(&storage_key).cloned())
    }

    async fn set_item_value(&self, id: &str, key: &str, value: Vec<u8>, _attributes: Option<HashMap<&str, &str>>) -> anyhow::Result<()> {
        let storage_key = format!("{}:{}", id, key);
        let mut data = self.data.lock().await;
        let data = data.as_mut().unwrap();
        data.insert(storage_key, value);
        Ok(())
    }

    async fn search_items(&self, id: &str, attributes: HashMap<&str, &str>) -> anyhow::Result<Vec<SecretItem>> {
        todo!()
    }
}

#[async_trait::async_trait]
impl SecretStoreInitializer for InMemorySecretStore {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(None)),
        }
    }

    async fn initialize(&mut self) -> anyhow::Result<()> {
        let mut data = self.data.lock().await;
        *data = Some(HashMap::new());
        println!("init secretstore");
        Ok(())
    }

}*/