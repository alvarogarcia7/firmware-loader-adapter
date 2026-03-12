use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_serial_port")]
    pub serial_port: String,

    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,

    #[serde(default = "default_origin_folder")]
    pub origin_folder: PathBuf,

    #[serde(default = "default_credentials_path")]
    pub credentials_path: PathBuf,
}

fn default_serial_port() -> String {
    if cfg!(target_os = "windows") {
        "COM3".to_string()
    } else {
        "/dev/ttyUSB0".to_string()
    }
}

fn default_baud_rate() -> u32 {
    115200
}

fn default_origin_folder() -> PathBuf {
    PathBuf::from(".")
}

fn default_credentials_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".secure-serial-transfer")
        .join("credentials.json")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            serial_port: default_serial_port(),
            baud_rate: default_baud_rate(),
            origin_folder: default_origin_folder(),
            credentials_path: default_credentials_path(),
        }
    }
}

impl Config {
    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_content =
            fs::read_to_string(config_path).context("Failed to read config file")?;

        let config: Config =
            toml::from_str(&config_content).context("Failed to parse config file")?;

        Ok(config)
    }

    #[allow(dead_code)]
    pub fn save(&self, config_path: &Path) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let config_content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(config_path, config_content).context("Failed to write config file")?;

        Ok(())
    }

    pub fn get_default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".secure-serial-transfer").join("config.toml")
    }
}
