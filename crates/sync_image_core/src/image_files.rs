use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};

use crate::build_user_dir;

const TIMESTAMP_FORMAT: &str = "%Y%m%d_%H%M%S_%3f";
const PNG_SUFFIX: &str = ".png";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedImage {
    pub file_name: String,
    pub upload_time: DateTime<Utc>,
    pub path: PathBuf,
}

pub fn format_upload_filename(upload_time: DateTime<Utc>) -> String {
    format!("{}.png", upload_time.format(TIMESTAMP_FORMAT))
}

pub fn parse_upload_filename(file_name: &str) -> Option<DateTime<Utc>> {
    let stem = file_name.strip_suffix(PNG_SUFFIX)?;
    let naive = NaiveDateTime::parse_from_str(stem, TIMESTAMP_FORMAT).ok()?;
    Some(DateTime::from_naive_utc_and_offset(naive, Utc))
}

pub fn find_latest_image(root: impl AsRef<Path>, user_name: &str) -> Result<Option<SelectedImage>> {
    let user_dir = build_user_dir(root, user_name)?;
    let entries = match fs::read_dir(&user_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read {}", user_dir.display()));
        }
    };

    let mut latest = None;
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(upload_time) = parse_upload_filename(&file_name) else {
            continue;
        };

        let candidate = SelectedImage {
            file_name,
            upload_time,
            path: entry.path(),
        };

        if latest
            .as_ref()
            .is_none_or(|current: &SelectedImage| candidate.file_name > current.file_name)
        {
            latest = Some(candidate);
        }
    }

    Ok(latest)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn timestamped_filenames_sort_by_time() {
        let early = parse_upload_filename("20260623_010203_004.png").expect("early");
        let late = parse_upload_filename("20260623_010203_104.png").expect("late");

        assert!(early < late);
        assert!("20260623_010203_004.png" < "20260623_010203_104.png");
    }

    #[test]
    fn rejects_non_timestamped_png() {
        assert!(parse_upload_filename("latest.png").is_none());
        assert!(parse_upload_filename("20260623_010203.png").is_none());
        assert!(parse_upload_filename("20260623_010203_004.jpg").is_none());
    }

    #[test]
    fn finds_latest_image_by_filename() {
        let root = temp_root();
        let user_dir = root.join("alice");
        fs::create_dir_all(&user_dir).expect("create user dir");
        File::create(user_dir.join("20260623_010203_004.png")).expect("early");
        File::create(user_dir.join("not-an-image.png")).expect("ignored");
        File::create(user_dir.join("20260623_010203_104.png")).expect("late");

        let selected = find_latest_image(&root, "alice")
            .expect("selection")
            .expect("latest image");

        assert_eq!(selected.file_name, "20260623_010203_104.png");

        fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("sync-image-core-test-{suffix}"))
    }
}
