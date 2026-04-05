use std::pin::Pin;
use generic_array::ArrayLength;
use aes::cipher::consts::U32;
use generic_array::GenericArray;
use anyhow::{Error};
use crate::encryption::{CryptoKey, EncryptedString};
use crate::encryption::symmetric::Aes256CbcHmacKey;

#[derive(Debug)]
pub enum HkdfExpandError {
    InvalidInputLength,
    InvalidOutputLength,
}

pub fn hkdf_expand<T: ArrayLength<u8>>(
    prk: &[u8],
    info: Option<&str>,
) -> Result<Pin<Box<GenericArray<u8, T>>>, HkdfExpandError> {
    let hkdf = hkdf::Hkdf::<sha2::Sha256>::from_prk(prk)
        .map_err(|_| HkdfExpandError::InvalidInputLength)?;
    let mut key = Box::<GenericArray<u8, T>>::default();

    let i = info.map(|i| i.as_bytes()).unwrap_or(&[]);
    hkdf.expand(i, &mut key)
        .map_err(|_| HkdfExpandError::InvalidOutputLength)?;

    Ok(Box::into_pin(key))
}

pub fn stretch_key(key: &Pin<Box<GenericArray<u8, U32>>>) -> Aes256CbcHmacKey {
    Aes256CbcHmacKey {
        // this is safe because the key length is always 32 bytes
        enc_key: hkdf_expand(key, Some("enc")).expect("HKDF expand to succeed"),
        mac_key: hkdf_expand(key, Some("mac")).expect("HKDF expand to succeed"),
    }
}

pub fn decrypt_field(field: String, key: &dyn CryptoKey) -> Result<Vec<u8>, Error> {
    match EncryptedString::try_from(field).and_then(|enc| enc.decrypt(key)) {
        Ok(bytes) => Ok(bytes),
        Err(e) => Err(e),
    }
}

pub fn decrypt_field_as_string(field: String, key: &dyn CryptoKey) -> Result<String, Error> {
    match decrypt_field(field, key) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).to_string()),
        Err(e) => Err(e),
    }
}