use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::psp::PspConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub currency: String,
    pub currency_symbol: String,
    pub providers: Vec<PspConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            currency: "EUR".to_string(),
            currency_symbol: "€".to_string(),
            providers: Vec::new(),
        }
    }
}

fn config_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("profit-cli");
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

pub fn load_config() -> Option<AppConfig> {
    let path = config_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = config_path();
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(path, data)?;
    Ok(())
}

