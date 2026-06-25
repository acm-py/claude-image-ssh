use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow, bail};
use sync_image_core::ClientConfig;

use crate::{
    client::{ClientActionState, connect_uploader_persisting},
    hotkey_runtime::{self, HotkeyRuntimeHandle},
    image_input,
    interaction::InteractionResponse,
    notify,
    upload::SftpUploader,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatus {
    Stopped,
    Running,
}

pub struct ClientRuntime {
    handle: HotkeyRuntimeHandle,
}

impl ClientRuntime {
    pub fn stop(self) -> Result<()> {
        self.handle.stop()
    }
}

pub fn start_runtime(
    config: ClientConfig,
    response: Option<InteractionResponse>,
    config_path: Option<PathBuf>,
) -> Result<ClientActionState<ClientRuntime>> {
    let hotkey = config.hotkey.parse()?;
    let uploader = match connect_uploader_persisting(config, response, config_path)? {
        ClientActionState::Ready(uploader) => uploader,
        ClientActionState::NeedsInteraction(request) => {
            return Ok(ClientActionState::NeedsInteraction(request));
        }
    };

    let uploader = Arc::new(Mutex::new(uploader));
    let handle = hotkey_runtime::start(&hotkey, move || {
        run_upload_cycle(&uploader);
    })?;

    Ok(ClientActionState::Ready(ClientRuntime { handle }))
}

fn run_upload_cycle(uploader: &Arc<Mutex<SftpUploader>>) {
    let result = image_input::capture_current_image().and_then(|image| {
        let mut uploader = uploader
            .lock()
            .map_err(|_| anyhow!("uploader mutex was poisoned"))?;
        uploader
            .upload_png(&image.png_bytes)
            .map(|upload| (image, upload))
    });

    match result {
        Ok((image, upload)) => notify::upload_success(&upload.file_name, image.source.label()),
        Err(err) => notify::upload_failure(&err.to_string()),
    }
}

pub struct RuntimeManager {
    runtime: Option<ClientRuntime>,
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self { runtime: None }
    }

    pub fn status(&self) -> RuntimeStatus {
        if self.runtime.is_some() {
            RuntimeStatus::Running
        } else {
            RuntimeStatus::Stopped
        }
    }

    pub fn start(
        &mut self,
        config: ClientConfig,
        response: Option<InteractionResponse>,
        config_path: Option<PathBuf>,
    ) -> Result<ClientActionState<RuntimeStatus>> {
        if self.runtime.is_some() {
            bail!("runtime is already running");
        }

        match start_runtime(config, response, config_path)? {
            ClientActionState::Ready(runtime) => {
                self.runtime = Some(runtime);
                Ok(ClientActionState::Ready(RuntimeStatus::Running))
            }
            ClientActionState::NeedsInteraction(request) => {
                Ok(ClientActionState::NeedsInteraction(request))
            }
        }
    }

    pub fn stop(&mut self) -> Result<RuntimeStatus> {
        let runtime = self
            .runtime
            .take()
            .ok_or_else(|| anyhow!("runtime is not running"))?;
        runtime.stop()?;
        Ok(RuntimeStatus::Stopped)
    }
}
