use anyhow::Error;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;

pub mod symmetric;
pub mod asymmetric;

pub trait CryptoKey {
    fn decrypt(&self, payload: &EncryptedString) -> anyhow::Result<Vec<u8>>;
}

#[derive(Debug)]
pub enum EncryptedString {
    Aes256cbcHmacSha256 {
        iv: [u8; 16],
        mac: [u8; 32],
        data: Vec<u8>,
    },
    Rsa2048OaepSha1 {
        data: Vec<u8>,
    }
}

fn split_enc_string(s: &str) -> (&str, Vec<&str>) {
    let header_parts: Vec<_> = s.split('.').collect();

    if header_parts.len() == 2 {
        (header_parts[0], header_parts[1].split('|').collect())
    } else {
        // Support legacy format with no header
        let parts: Vec<_> = s.split('|').collect();
        if parts.len() == 3 {
            ("1", parts) // Aes128Cbc_HmacSha256_B64
        } else {
            ("0", parts) // Aes256Cbc_B64
        }
    }
}

impl EncryptedString {
    fn parse(value: String) -> Result<EncryptedString, Error> {
        let (enc_type, parts) = split_enc_string(&value);

        match (enc_type, parts.len()) {
            ("4", 1) => {
                let data = BASE64_STANDARD.decode(parts[0])?;

                Ok(EncryptedString::Rsa2048OaepSha1 { data })
            }
            ("2", 3) => {
                let mut iv = [0u8; 16];
                let mut mac = [0u8; 32];

                let iv_len = BASE64_STANDARD.decode_slice(parts[0], &mut iv)?;
                if iv_len != 16 {
                    return Err(anyhow::anyhow!("Invalid IV length"));
                }

                let data = BASE64_STANDARD.decode(parts[1])?;

                let mac_len = BASE64_STANDARD.decode_slice(parts[2], &mut mac)?;
                if mac_len != 32 {
                    return Err(anyhow::anyhow!("Invalid MAC length"));
                }

                Ok(EncryptedString::Aes256cbcHmacSha256 { iv, mac, data })
            }
            _ => Err(anyhow::anyhow!("Invalid type")),
        }
    }

    pub fn decrypt(&self, key: &dyn CryptoKey) -> Result<Vec<u8>, Error> {
        key.decrypt(self)
    }
}

impl TryFrom<String> for EncryptedString {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        EncryptedString::parse(value)
    }
}
