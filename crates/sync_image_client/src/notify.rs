pub fn upload_success(file_name: &str, source: &str) {
    let message = format!("uploaded {file_name} from {source}");
    println!("success: {message}");
    toast("Claude image uploaded", &message);
}

pub fn upload_failure(error: &str) {
    eprintln!("upload failed: {error}");
    toast("Claude image upload failed", error);
}

#[cfg(windows)]
fn toast(title: &str, message: &str) {
    use winrt_notification::{Duration, Toast};

    if let Err(err) = Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(message)
        .duration(Duration::Short)
        .show()
    {
        eprintln!("toast notification failed: {err}");
    }
}

#[cfg(not(windows))]
fn toast(_title: &str, _message: &str) {}
