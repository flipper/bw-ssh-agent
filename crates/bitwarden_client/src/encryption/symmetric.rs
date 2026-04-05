use aes::cipher::KeyIvInit;
use std::pin::Pin;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::BlockDecryptMut;
use aes::cipher::consts::U32;
use anyhow::Error;
use ctutils::CtEq;
use generic_array::GenericArray;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use crate::encryption::{CryptoKey, EncryptedString};

#[derive(Debug)]
pub struct Aes256CbcHmacKey {
    pub enc_key: Pin<Box<GenericArray<u8, U32>>>,
    pub mac_key: Pin<Box<GenericArray<u8, U32>>>,
}

impl Aes256CbcHmacKey {
    pub fn new(enc: [u8; 32], mac: [u8; 32]) -> Self {
        let enc: &GenericArray<u8, U32> = GenericArray::from_slice(&enc);
        let mac: &GenericArray<u8, U32> = GenericArray::from_slice(&mac);

        Self {
            enc_key: Pin::new(Box::new(*enc)),
            mac_key: Pin::new(Box::new(*mac)),
        }
    }

    fn generate_mac(mac_key: &[u8], iv: &[u8], data: &[u8]) -> [u8; 32] {
        let mut hmac: Hmac<Sha256> =
            KeyInit::new_from_slice(mac_key).expect("new_from_slice cannot fail");

        hmac.update(iv);
        hmac.update(data);

        let mac: [u8; 32] = (*hmac.finalize().into_bytes())
            .try_into()
            // This is safe because Pbkdf2Sha256Hmac output size is always 32 bytes
            .expect("HMAC output size to be correct");

        mac
    }

    fn decrypt_aes256(
        iv: &[u8; 16],
        data: Vec<u8>,
        key: &GenericArray<u8, U32>,
    ) -> Result<Vec<u8>, Error> {
        // Decrypt data
        let iv = GenericArray::from_slice(iv);
        let mut data = data;
        let decrypted_key_slice = cbc::Decryptor::<aes::Aes256>::new(key, iv)
            .decrypt_padded_mut::<Pkcs7>(&mut data)
            .map_err(|e| anyhow::anyhow!(e))?;

        // Data is decrypted in place and returns a subslice of the original Vec, to avoid cloning it,
        // we truncate to the subslice length
        let decrypted_len = decrypted_key_slice.len();
        data.truncate(decrypted_len);

        Ok(data)
    }
}

impl CryptoKey for Aes256CbcHmacKey {

    fn decrypt(&self, payload: &EncryptedString) -> anyhow::Result<Vec<u8>> {
        match payload {
            EncryptedString::Aes256cbcHmacSha256 { iv, mac, data } => {
                let computed_mac = Self::generate_mac(&self.mac_key, iv, data.as_slice());

                if computed_mac.ct_ne(&mac).into() {
                    return Err(anyhow::anyhow!("mac is not correct"));
                }

                let decrypted = Self::decrypt_aes256(iv, data.clone(), &self.enc_key)?;
                Ok(decrypted)
            }
            _ => anyhow::bail!("this key cannot decrypt this type of EncryptedString"),
        }
    }
}