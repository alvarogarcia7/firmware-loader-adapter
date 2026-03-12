use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use scrypt::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Params, Scrypt,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const SCRYPT_LOG_N: u8 = 15;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password_hash: String,
    pub salt: String,
}

impl Credentials {
    pub fn new(username: String, password: &str) -> Result<Self> {
        let salt = SaltString::generate(&mut OsRng);
        let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LEN)
            .context("Failed to create scrypt parameters")?;
        
        let password_hash = Scrypt
            .hash_password_customized(password.as_bytes(), None, None, params, &salt)
            .context("Failed to hash password")?
            .to_string();

        Ok(Self {
            username,
            password_hash,
            salt: salt.to_string(),
        })
    }
}

pub fn store_credentials(credentials: &Credentials, password: &str, file_path: &Path) -> Result<()> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LEN)
        .context("Failed to create scrypt parameters")?;
    
    let password_hash = Scrypt
        .hash_password_customized(password.as_bytes(), None, None, params, &salt)
        .context("Failed to derive key from password")?;
    
    let hash_output = password_hash.hash.context("No hash generated")?;
    let key_bytes = hash_output.as_bytes();
    
    if key_bytes.len() < KEY_LEN {
        return Err(anyhow::anyhow!("Derived key too short"));
    }
    
    let mut encryption_key = [0u8; KEY_LEN];
    encryption_key.copy_from_slice(&key_bytes[..KEY_LEN]);

    let cipher = Aes256Gcm::new(&encryption_key.into());
    let nonce_bytes = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let credentials_json = serde_json::to_string(credentials)
        .context("Failed to serialize credentials")?;
    
    let encrypted_data = cipher
        .encrypt(nonce, credentials_json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let storage_data = StoredCredentials {
        salt: salt.to_string(),
        nonce: hex::encode(nonce_bytes),
        encrypted_credentials: hex::encode(&encrypted_data),
    };

    let storage_json = serde_json::to_string_pretty(&storage_data)
        .context("Failed to serialize storage data")?;
    
    fs::write(file_path, storage_json)
        .context("Failed to write credentials file")?;

    Ok(())
}

pub fn verify_credentials(username: &str, password: &str, file_path: &Path) -> Result<bool> {
    let storage_json = fs::read_to_string(file_path)
        .context("Failed to read credentials file")?;
    
    let storage_data: StoredCredentials = serde_json::from_str(&storage_json)
        .context("Failed to parse storage data")?;

    let salt = SaltString::from_b64(&storage_data.salt)
        .context("Invalid salt format")?;
    
    let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LEN)
        .context("Failed to create scrypt parameters")?;
    
    let password_hash = Scrypt
        .hash_password_customized(password.as_bytes(), None, None, params, &salt)
        .context("Failed to derive key from password")?;
    
    let hash_output = password_hash.hash.context("No hash generated")?;
    let key_bytes = hash_output.as_bytes();
    
    if key_bytes.len() < KEY_LEN {
        return Err(anyhow::anyhow!("Derived key too short"));
    }
    
    let mut encryption_key = [0u8; KEY_LEN];
    encryption_key.copy_from_slice(&key_bytes[..KEY_LEN]);

    let cipher = Aes256Gcm::new(&encryption_key.into());
    
    let nonce_bytes = hex::decode(&storage_data.nonce)
        .context("Invalid nonce format")?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let encrypted_data = hex::decode(&storage_data.encrypted_credentials)
        .context("Invalid encrypted data format")?;

    let decrypted_data = cipher
        .decrypt(nonce, encrypted_data.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed - invalid password or corrupted data"))?;

    let credentials: Credentials = serde_json::from_slice(&decrypted_data)
        .context("Failed to parse credentials")?;

    if credentials.username != username {
        return Ok(false);
    }

    let stored_hash = PasswordHash::new(&credentials.password_hash)
        .context("Invalid password hash format")?;
    
    Ok(Scrypt.verify_password(password.as_bytes(), &stored_hash).is_ok())
}

fn generate_nonce() -> [u8; 12] {
    use aes_gcm::aead::rand_core::RngCore;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

#[derive(Serialize, Deserialize)]
struct StoredCredentials {
    salt: String,
    nonce: String,
    encrypted_credentials: String,
}

#[allow(dead_code)]
pub struct AuthManager {
    key: [u8; 32],
}

#[allow(dead_code)]
impl AuthManager {
    pub fn new(password: &str) -> Result<Self> {
        let salt = SaltString::generate(&mut OsRng);
        let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LEN)
            .context("Failed to create scrypt parameters")?;
        
        let password_hash = Scrypt
            .hash_password_customized(password.as_bytes(), None, None, params, &salt)
            .context("Failed to hash password")?;

        let hash_bytes = password_hash.hash.context("No hash generated")?;
        let key_bytes = hash_bytes.as_bytes();
        
        if key_bytes.len() < KEY_LEN {
            return Err(anyhow::anyhow!("Derived key too short"));
        }
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes[..KEY_LEN]);

        Ok(Self { key })
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce_bytes = generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let mut encrypted = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        let mut result = nonce_bytes.to_vec();
        result.append(&mut encrypted);
        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 12 {
            return Err(anyhow::anyhow!("Data too short to contain nonce"));
        }
        
        let (nonce_bytes, encrypted_data) = data.split_at(12);
        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce = Nonce::from_slice(nonce_bytes);
        
        cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    pub fn get_key(&self) -> &[u8; 32] {
        &self.key
    }
}
