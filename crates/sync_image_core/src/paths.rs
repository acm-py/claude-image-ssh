use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub fn validate_user_name(user_name: &str) -> Result<()> {
    if user_name.is_empty() {
        bail!("user_name is required");
    }

    if user_name == "." || user_name == ".." {
        bail!("user_name cannot be '{user_name}'");
    }

    if user_name.contains('/') || user_name.contains('\\') {
        bail!("user_name must be a single path component");
    }

    if !user_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
    {
        bail!("user_name may only contain ASCII letters, digits, '.', '_' and '-'");
    }

    Ok(())
}

pub fn build_user_dir(root: impl AsRef<Path>, user_name: &str) -> Result<PathBuf> {
    validate_user_name(user_name)?;
    Ok(root.as_ref().join(user_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_single_component_user_path() {
        let path = build_user_dir("/data/claude-images", "alice.dev").expect("path");
        assert_eq!(path, PathBuf::from("/data/claude-images/alice.dev"));
    }

    #[test]
    fn rejects_traversal_user_path() {
        let err = build_user_dir("/data/claude-images", "../alice").expect_err("invalid");
        assert!(err.to_string().contains("single path component"));
    }
}
