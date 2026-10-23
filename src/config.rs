use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub github: GithubConfig,
    pub ui: UiConfig,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ThemeConfig {
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub text: Option<String>,
    pub background: Option<String>,
    pub border: Option<String>,
    pub highlight: Option<String>,
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

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("primary", self.theme.primary.as_deref()),
            ("secondary", self.theme.secondary.as_deref()),
            ("text", self.theme.text.as_deref()),
            ("background", self.theme.background.as_deref()),
            ("border", self.theme.border.as_deref()),
            ("highlight", self.theme.highlight.as_deref()),
        ] {
            if let Some(value) = value {
                parse_hex_color(value).with_context(|| format!("Invalid [theme].{name} color"))?;
            }
        }

        if self.ui.theme.eq_ignore_ascii_case("custom") && !self.theme.has_colors() {
            anyhow::bail!("ui.theme is set to \"custom\", but no [theme] colors are configured");
        }
        Ok(())
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

impl ThemeConfig {
    pub fn has_colors(&self) -> bool {
        self.primary.is_some()
            || self.secondary.is_some()
            || self.text.is_some()
            || self.background.is_some()
            || self.border.is_some()
            || self.highlight.is_some()
    }
}

pub fn parse_hex_color(value: &str) -> Result<(u8, u8, u8)> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("expected a six-digit hex color such as #CBA6F7");
    }
    let red = u8::from_str_radix(&hex[0..2], 16)?;
    let green = u8::from_str_radix(&hex[2..4], 16)?;
    let blue = u8::from_str_radix(&hex[4..6], 16)?;
    Ok((red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::{parse_hex_color, Config};

    #[test]
    fn parses_hex_rgb_colors() {
        assert_eq!(parse_hex_color("#CBA6F7").unwrap(), (203, 166, 247));
        assert_eq!(parse_hex_color("000000").unwrap(), (0, 0, 0));
    }

    #[test]
    fn rejects_invalid_hex_colors() {
        assert!(parse_hex_color("#12FG00").is_err());
        assert!(parse_hex_color("#FFF").is_err());
    }

    #[test]
    fn validates_custom_theme_colors() {
        let mut config = Config::default();
        config.ui.theme = "custom".to_string();
        config.theme.primary = Some("#CBA6F7".to_string());
        assert!(config.validate().is_ok());

        config.theme.primary = Some("purple".to_string());
        assert!(config.validate().is_err());
    }
}
