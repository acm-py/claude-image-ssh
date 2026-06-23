mod cli;
mod hotkey_runtime;
mod image_input;
mod notify;
mod upload;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use sync_image_core::ClientConfig;
use upload::SftpUploader;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = ClientConfig::load(cli.config)?;

    match cli.command.unwrap_or(Command::Run) {
        Command::Run => run_client(config),
        Command::Check => run_check(config),
    }
}

fn run_client(config: ClientConfig) -> Result<()> {
    let hotkey = config.hotkey.parse()?;
    let mut uploader = SftpUploader::connect(config)?;

    println!("sync-image-client is running. Press {hotkey} to upload the current image.");
    hotkey_runtime::run(&hotkey, move || {
        let result = image_input::capture_current_image().and_then(|image| {
            uploader
                .upload_png(&image.png_bytes)
                .map(|upload| (image, upload))
        });

        match result {
            Ok((image, upload)) => notify::upload_success(&upload.file_name, image.source.label()),
            Err(err) => notify::upload_failure(&err.to_string()),
        }
    })
}

fn run_check(config: ClientConfig) -> Result<()> {
    let mut uploader = SftpUploader::connect(config)?;
    uploader.check_remote_write_access()?;
    println!("check passed: configuration, SSH/SFTP, host key and remote write access are valid");
    Ok(())
}
