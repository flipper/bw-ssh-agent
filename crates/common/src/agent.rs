use crate::{SecretStore, APP_ID, DEFAULT_STORE};
use anyhow::anyhow;
use signature::Signer;
use ssh_agent_lib::agent::Session;
use ssh_agent_lib::error::AgentError;
use ssh_agent_lib::proto::extension::{QueryResponse, SessionBind};
use ssh_agent_lib::proto::{Extension, Identity, SignRequest};
use ssh_agent_lib::ssh_key::{HashAlg, PrivateKey, PublicKey, Signature};
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct Agent;

#[ssh_agent_lib::async_trait]
impl Session for Agent {
    async fn request_identities(&mut self) -> Result<Vec<Identity>, AgentError> {
        let store = DEFAULT_STORE.read().await;

        let mut filter = HashMap::new();
        filter.insert("type", "pub-key");

        let results = store
            .search_items(APP_ID, filter)
            .await
            .map_err(|e| AgentError::Other(e.into()))?;

        let mut public_keys: Vec<PublicKey> = vec![];

        for item in results {
            let value = item.get().await.map_err(|e| AgentError::Other(e.into()))?;
            let attributes = item
                .get_attributes()
                .await
                .map_err(|e| AgentError::Other(e.into()))?;

            let public_key = match (value, attributes) {
                (Some(value), Some(attrs)) => {
                    let value = String::from_utf8(value).map_err(|e| AgentError::Other(e.into()))?;

                    match PublicKey::from_openssh(value.as_str()) {
                        Ok(mut pk) => {
                            let comment = attrs.get("comment");
                            if let Some(comment) = comment {
                                pk.set_comment(comment);
                            }
                            pk
                        },
                        Err(e) => return Err(AgentError::Other(e.into())),
                    }
                },
                _ => continue
            };

            public_keys.push(public_key);
        }

        let identities = public_keys.iter().map(|pk| {
            Identity {
                pubkey: pk.key_data().clone(),
                comment: pk.comment().to_string(),
            }
        }).collect();

        Ok(identities)
    }

    async fn sign(&mut self, request: SignRequest) -> Result<Signature, AgentError> {
        let store = DEFAULT_STORE.read().await;

        let request_fingerprint = request.pubkey.fingerprint(HashAlg::Sha256).to_string();

        let mut filter = HashMap::new();
        filter.insert("type", "pri-key");
        filter.insert("fingerprint", request_fingerprint.as_str());

        let results = store
            .search_items(APP_ID, filter)
            .await
            .map_err(|e| AgentError::Other(e.into()))?;

        let item = results.first();

        match item {
            None => Err(AgentError::Other(anyhow!("No private key found for this request").into())),
            Some(item) => {
                let private_key = item.get().await.map_err(|e| AgentError::Other(e.into()))?;

                match private_key {
                    Some(private_key) => {
                        let private_key = PrivateKey::from_openssh(&private_key)
                            .map_err(|e| AgentError::Other(e.into()))?;

                        private_key
                            .try_sign(&request.data)
                            .map_err(|e| AgentError::other(e))
                    }
                    None => Err(AgentError::Other(anyhow!("No private key found").into())),
                }
            }
        }
    }

    async fn extension(&mut self, extension: Extension) -> Result<Option<Extension>, AgentError> {
        match extension.name.as_str() {
            "query" => {
                let response = Extension::new_message(QueryResponse {
                    extensions: vec!["query".into(), "session-bind@openssh.com".into()],
                })?;
                Ok(Some(response))
            }
            "session-bind@openssh.com" => match extension.parse_message::<SessionBind>()? {
                Some(bind) => {
                    bind.verify_signature()
                        .map_err(|_| AgentError::ExtensionFailure)?;
                    Ok(None)
                }
                None => Err(AgentError::Failure),
            },
            _ => Err(AgentError::Failure),
        }
    }
}
