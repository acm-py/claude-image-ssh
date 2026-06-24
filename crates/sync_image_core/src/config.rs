use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{Hotkey, validate_user_name};

pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+U";
pub const DEFAULT_CONFIG_RELATIVE_PATH: &str = "claude-image-sync/config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub user_name: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    pub upload: UploadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub user: String,
    pub private_key_path: PathBuf,
    pub shared_image_root: String,
}

impl ClientConfig {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let path = path.unwrap_or_else(default_config_path);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let config = Self::from_toml(&raw)
            .with_context(|| format!("failed to parse config {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_toml(raw: &str) -> Result<Self> {
        let config: Self = toml::from_str(raw)?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml(&self) -> Result<String> {
        self.validate()?;
        toml::to_string_pretty(self).context("failed to serialize client config")
    }

    pub fn save(&self, path: Option<PathBuf>) -> Result<PathBuf> {
        let path = path.unwrap_or_else(default_config_path);
        let raw = self.to_toml()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }

        fs::write(&path, raw)
            .with_context(|| format!("failed to write config {}", path.display()))?;
        Ok(path)
    }

    pub fn validate(&self) -> Result<()> {
        validate_user_name(&self.user_name)?;
        self.hotkey.parse::<Hotkey>()?;
        require_non_empty("upload.host", &self.upload.host)?;
        require_non_empty("upload.user", &self.upload.user)?;
        require_non_empty("upload.shared_image_root", &self.upload.shared_image_root)?;

        if self.upload.port == 0 {
            bail!("upload.port must be between 1 and 65535");
        }

        if self.upload.private_key_path.as_os_str().is_empty() {
            bail!("upload.private_key_path is required");
        }

        Ok(())
    }
}

pub fn default_config_path() -> PathBuf {
    if let Some(appdata) = env::var_os("APPDATA") {
        return PathBuf::from(appdata).join(DEFAULT_CONFIG_RELATIVE_PATH);
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join(DEFAULT_CONFIG_RELATIVE_PATH);
    }

    PathBuf::from(DEFAULT_CONFIG_RELATIVE_PATH)
}

fn default_hotkey() -> String {
    DEFAULT_HOTKEY.to_string()
}

fn default_port() -> u16 {
    22
}

fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} is required");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_valid_config() {
        let config = ClientConfig::from_toml(
            r#"
user_name = "alice"

[upload]
host = "upload.internal"
user = "claude-upload"
private_key_path = "C:\\Users\\alice\\.ssh\\id_ed25519"
shared_image_root = "/mnt/xy_internel/share/claude"
"#,
        )
        .expect("config should parse");

        assert_eq!(config.hotkey, DEFAULT_HOTKEY);
        assert_eq!(config.upload.port, 22);
    }

    #[test]
    fn rejects_path_like_user_name() {
        let err = ClientConfig::from_toml(
            r#"
user_name = "../alice"

[upload]
host = "upload.internal"
user = "claude-upload"
private_key_path = "id"
shared_image_root = "/mnt/xy_internel/share/claude"
"#,
        )
        .expect_err("path-like user name should fail");

        assert!(err.to_string().contains("user_name"));
    }

    #[test]
    fn serializes_with_defaults() {
        let config = ClientConfig::from_toml(
            r#"
user_name = "alice"

[upload]
host = "upload.internal"
user = "claude-upload"
private_key_path = "C:\\Users\\alice\\.ssh\\id_ed25519"
shared_image_root = "/mnt/xy_internel/share/claude"
"#,
        )
        .expect("config should parse");

        let raw = config.to_toml().expect("config should serialize");

        assert!(raw.contains("hotkey = \"Ctrl+Alt+U\""));
        assert!(raw.contains("port = 22"));
    }
}
