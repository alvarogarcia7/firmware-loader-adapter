use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const SESSION_VALIDITY_SECONDS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub username: String,
    pub timestamp: u64,
    pub port: String,
    pub baud_rate: u32,
}

impl Session {
    pub fn new(username: String, port: String, baud_rate: u32) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            username,
            timestamp,
            port,
            baud_rate,
        }
    }

    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.timestamp < SESSION_VALIDITY_SECONDS
    }
}

pub struct SessionManager {
    session_file: PathBuf,
}

impl SessionManager {
    pub fn new(session_file: PathBuf) -> Self {
        Self { session_file }
    }

    pub fn save_session(&self, session: &Session) -> Result<()> {
        let session_json = serde_json::to_string_pretty(session)
            .context("Failed to serialize session")?;
        
        if let Some(parent) = self.session_file.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create session directory")?;
        }
        
        fs::write(&self.session_file, session_json)
            .context("Failed to write session file")?;
        
        Ok(())
    }

    pub fn load_session(&self) -> Result<Option<Session>> {
        if !self.session_file.exists() {
            return Ok(None);
        }

        let session_json = fs::read_to_string(&self.session_file)
            .context("Failed to read session file")?;
        
        let session: Session = serde_json::from_str(&session_json)
            .context("Failed to parse session data")?;
        
        if session.is_valid() {
            Ok(Some(session))
        } else {
            self.clear_session()?;
            Ok(None)
        }
    }

    pub fn clear_session(&self) -> Result<()> {
        if self.session_file.exists() {
            fs::remove_file(&self.session_file)
                .context("Failed to remove session file")?;
        }
        Ok(())
    }

    pub fn get_session_or_error(&self) -> Result<Session> {
        self.load_session()?
            .ok_or_else(|| anyhow::anyhow!("No active session. Please login first."))
    }
}

pub fn get_default_session_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".secure-serial-transfer").join("session.json")
}

pub fn get_default_credentials_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".secure-serial-transfer").join("credentials.json")
}
