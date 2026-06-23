use std::{io::Cursor, path::Path};

use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageFormat};

#[derive(Debug, Clone)]
pub struct CapturedImage {
    pub png_bytes: Vec<u8>,
    pub source: ImageSource,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum ImageSource {
    ClipboardImage,
    CopiedFile,
}

impl ImageSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClipboardImage => "clipboard image",
            Self::CopiedFile => "copied file",
        }
    }
}

pub fn capture_current_image() -> Result<CapturedImage> {
    platform::capture_current_image()
}

#[cfg_attr(not(windows), allow(dead_code))]
fn encode_png(image: DynamicImage) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, ImageFormat::Png)?;
    Ok(output.into_inner())
}

#[cfg_attr(not(windows), allow(dead_code))]
fn normalize_file_to_png(path: &Path) -> Result<Vec<u8>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
        bail!("unsupported copied file type; only PNG and JPEG are accepted");
    }

    let image =
        image::open(path).with_context(|| format!("failed to decode {}", path.display()))?;
    encode_png(image)
}

#[cfg(windows)]
mod platform {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf, ptr};

    use anyhow::{Context, Result, bail};
    use arboard::Clipboard;
    use image::{DynamicImage, RgbaImage};
    use windows_sys::Win32::{
        System::{
            DataExchange::{
                CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
            },
            Ole::CF_HDROP,
        },
        UI::Shell::DragQueryFileW,
    };

    use super::{CapturedImage, ImageSource, encode_png, normalize_file_to_png};

    const DRAG_QUERY_FILE_COUNT: u32 = 0xFFFF_FFFF;

    pub fn capture_current_image() -> Result<CapturedImage> {
        if let Some(path) = copied_file_path()? {
            return Ok(CapturedImage {
                png_bytes: normalize_file_to_png(&path)?,
                source: ImageSource::CopiedFile,
            });
        }

        let mut clipboard = Clipboard::new().context("failed to open clipboard")?;
        let image = clipboard
            .get_image()
            .context("clipboard does not contain a supported image")?;
        let rgba = RgbaImage::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
        .context("clipboard image buffer has an unexpected size")?;

        Ok(CapturedImage {
            png_bytes: encode_png(DynamicImage::ImageRgba8(rgba))?,
            source: ImageSource::ClipboardImage,
        })
    }

    fn copied_file_path() -> Result<Option<PathBuf>> {
        let _guard = ClipboardGuard::open()?;
        unsafe {
            let file_drop_format = u32::from(CF_HDROP);
            if IsClipboardFormatAvailable(file_drop_format) == 0 {
                return Ok(None);
            }

            let handle = GetClipboardData(file_drop_format);
            if handle.is_null() {
                return Ok(None);
            }

            let count = DragQueryFileW(handle, DRAG_QUERY_FILE_COUNT, ptr::null_mut(), 0);
            if count == 0 {
                return Ok(None);
            }
            if count > 1 {
                bail!("multiple copied files are not supported");
            }

            let len = DragQueryFileW(handle, 0, ptr::null_mut(), 0);
            let mut buffer = vec![0u16; len as usize + 1];
            DragQueryFileW(handle, 0, buffer.as_mut_ptr(), buffer.len() as u32);
            buffer.truncate(len as usize);

            Ok(Some(PathBuf::from(OsString::from_wide(&buffer))))
        }
    }

    struct ClipboardGuard;

    impl ClipboardGuard {
        fn open() -> Result<Self> {
            unsafe {
                if OpenClipboard(ptr::null_mut()) == 0 {
                    bail!("failed to open clipboard");
                }
            }
            Ok(Self)
        }
    }

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use anyhow::{Result, bail};

    use super::CapturedImage;

    pub fn capture_current_image() -> Result<CapturedImage> {
        bail!("clipboard upload is only supported on Windows in this MVP")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_file_extension_before_decode() {
        let err = normalize_file_to_png(Path::new("example.webp")).expect_err("unsupported");
        assert!(err.to_string().contains("PNG and JPEG"));
    }
}
