use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub github: GithubConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GithubConfig {
    pub token: Option<String>,
    pub username: String,
    pub api_url: String,
    pub repos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub refresh_interval_secs: u64,
    pub theme: String,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            token: None,
            username: String::new(),
            api_url: "https://api.github.com".to_string(),
            repos: vec!["weaztoven/cleaner-cli".to_string()],
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 60,
            theme: "catppuccin".to_string(),
        }
    }
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("Could not find the config directory")?;
        Ok(dir.join("git-pulse").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        let mut config = if path.exists() {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("Could not read {}", path.display()))?;
            toml::from_str(&raw).with_context(|| format!("{} is not valid TOML", path.display()))?
        } else {
            let cfg = Config::default();
            cfg.save()?;
            cfg
        };

        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            config.github.token = Some(token);
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self).context("Could not serialize the config")?;
        fs::write(&path, raw).with_context(|| format!("Could not write {}", path.display()))?;
        Ok(())
    }
}
