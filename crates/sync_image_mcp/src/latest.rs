use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use sync_image_core::{SelectedImage, find_latest_image};

#[derive(Debug, Clone)]
pub struct LatestImage {
    pub selected: SelectedImage,
    pub data_base64: String,
}

impl LatestImage {
    pub fn upload_time_text(&self) -> String {
        self.selected.upload_time.to_rfc3339()
    }
}

pub fn load_latest_image(root: impl AsRef<Path>, user_name: &str) -> Result<LatestImage> {
    let Some(selected) = find_latest_image(root, user_name)? else {
        bail!("no timestamped PNG uploads found for user '{user_name}'");
    };

    let bytes = fs::read(&selected.path)
        .with_context(|| format!("failed to read {}", selected.path.display()))?;

    Ok(LatestImage {
        selected,
        data_base64: STANDARD.encode(bytes),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn loads_latest_image_data() {
        let root = temp_root();
        let user_dir = root.join("alice");
        fs::create_dir_all(&user_dir).expect("create dir");
        File::create(user_dir.join("20260623_010203_004.png")).expect("early");
        let mut late = File::create(user_dir.join("20260623_010203_104.png")).expect("late");
        late.write_all(b"png-bytes").expect("write");

        let latest = load_latest_image(&root, "alice").expect("latest");

        assert_eq!(latest.selected.file_name, "20260623_010203_104.png");
        assert_eq!(latest.data_base64, "cG5nLWJ5dGVz");

        fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("sync-image-mcp-test-{suffix}"))
    }
}
