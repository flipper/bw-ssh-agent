use anyhow::Error;
use rsa::pkcs8::DecodePrivateKey;
use rsa::{Oaep, RsaPrivateKey};
use rsa::traits::PaddingScheme;
use sha1::Sha1;
use crate::encryption::{CryptoKey, EncryptedString};


pub struct BWRsaPrivateKey {
    inner: RsaPrivateKey
}

impl BWRsaPrivateKey {
    pub fn new(pkcs8_der: Vec<u8>) -> Result<Self, Error> {
        let private_key = RsaPrivateKey::from_pkcs8_der(&pkcs8_der)?;

        Ok(Self {
            inner: private_key
        })
    }
}

impl CryptoKey for BWRsaPrivateKey {
    fn decrypt(&self, payload: &EncryptedString) -> anyhow::Result<Vec<u8>> {
        match payload {
            EncryptedString::Rsa2048OaepSha1 { data} => {
                let mut rng = rand::rng();
                let r = Oaep::<Sha1>::new().decrypt(Some(&mut rng), &self.inner, data.as_slice())?;
                Ok(r)
            }
            _ => anyhow::bail!("this key cannot decrypt this type of EncryptedString"),
        }
    }
}