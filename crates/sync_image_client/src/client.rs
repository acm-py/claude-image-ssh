use anyhow::{Result, anyhow, bail};
use sync_image_core::ClientConfig;

use crate::{
    hotkey_runtime, image_input,
    interaction::{InteractionRequest, InteractionResponse},
    notify,
    upload::{ConnectError, SftpUploader},
};

#[derive(Debug)]
pub enum ClientActionState<T> {
    Ready(T),
    NeedsInteraction(InteractionRequest),
}

pub fn connect_uploader(
    config: ClientConfig,
    response: Option<InteractionResponse>,
) -> Result<ClientActionState<SftpUploader>> {
    match SftpUploader::connect(config, response) {
        Ok(uploader) => Ok(ClientActionState::Ready(uploader)),
        Err(ConnectError::NeedsInteraction(request)) => {
            Ok(ClientActionState::NeedsInteraction(request))
        }
        Err(ConnectError::Failed(err)) => Err(err),
    }
}

pub fn run_check(
    config: ClientConfig,
    response: Option<InteractionResponse>,
) -> Result<ClientActionState<()>> {
    let uploader = match connect_uploader(config, response)? {
        ClientActionState::Ready(uploader) => uploader,
        ClientActionState::NeedsInteraction(request) => {
            return Ok(ClientActionState::NeedsInteraction(request));
        }
    };

    let mut uploader = uploader;
    uploader.check_remote_write_access()?;
    Ok(ClientActionState::Ready(()))
}

pub fn run_client_once(
    config: ClientConfig,
    response: Option<InteractionResponse>,
) -> Result<ClientActionState<()>> {
    let hotkey = config.hotkey.parse()?;
    let uploader = match connect_uploader(config, response)? {
        ClientActionState::Ready(uploader) => uploader,
        ClientActionState::NeedsInteraction(request) => {
            return Ok(ClientActionState::NeedsInteraction(request));
        }
    };

    let mut uploader = uploader;
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
    })?;

    Ok(ClientActionState::Ready(()))
}

pub fn resolve_interaction(
    request: &InteractionRequest,
    response: Option<InteractionResponse>,
) -> Result<InteractionResponse> {
    match (request, response) {
        (InteractionRequest::TrustHostKey(_), Some(InteractionResponse::TrustHostKey(value))) => {
            Ok(InteractionResponse::TrustHostKey(value))
        }
        (
            InteractionRequest::PrivateKeyPassphrase(_),
            Some(InteractionResponse::PrivateKeyPassphrase(value)),
        ) => Ok(InteractionResponse::PrivateKeyPassphrase(value)),
        _ => Err(anyhow!("missing or mismatched interaction response")),
    }
}

pub fn trust_host_key_response(
    request: &InteractionRequest,
    trusted: bool,
) -> Result<InteractionResponse> {
    if matches!(request, InteractionRequest::TrustHostKey(_)) {
        Ok(InteractionResponse::TrustHostKey(trusted))
    } else {
        bail!("expected trust-host-key interaction")
    }
}

pub fn passphrase_response(
    request: &InteractionRequest,
    passphrase: String,
) -> Result<InteractionResponse> {
    if matches!(request, InteractionRequest::PrivateKeyPassphrase(_)) {
        Ok(InteractionResponse::PrivateKeyPassphrase(passphrase))
    } else {
        bail!("expected passphrase interaction")
    }
}
