use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use scrypt::{
    password_hash::{PasswordHasher, SaltString},
    Scrypt,
};

#[allow(dead_code)]
pub struct AuthManager {
    key: [u8; 32],
}

#[allow(dead_code)]
impl AuthManager {
    pub fn new(password: &str) -> Result<Self> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Scrypt
            .hash_password(password.as_bytes(), &salt)
            .context("Failed to hash password")?;

        let hash_bytes = password_hash.hash.context("No hash generated")?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);

        Ok(Self { key })
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce = Nonce::from_slice(&[0u8; 12]);
        
        cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce = Nonce::from_slice(&[0u8; 12]);
        
        cipher
            .decrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    pub fn get_key(&self) -> &[u8; 32] {
        &self.key
    }
}
