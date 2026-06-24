use std::{path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use sync_image_client::{
    client::{ClientActionState, run_check},
    interaction::{InteractionRequest, InteractionResponse},
    runtime::{RuntimeManager, RuntimeStatus},
};
use sync_image_core::{ClientConfig, UploadConfig, config::default_config_path};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigDraft {
    user_name: String,
    hotkey: String,
    upload: UploadDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadDraft {
    host: String,
    port: u16,
    user: String,
    private_key_path: String,
    shared_image_root: String,
}

#[derive(Debug, Serialize)]
struct LoadConfigResponse {
    config: ConfigDraft,
    path: String,
    exists: bool,
}

#[derive(Debug, Serialize)]
struct SaveConfigResponse {
    path: String,
}

#[derive(Debug, Serialize)]
struct RuntimeStatusResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum CommandResponse {
    Ok {
        message: String,
        runtime_status: Option<&'static str>,
    },
    NeedsInteraction {
        request: InteractionRequestDto,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InteractionRequestDto {
    TrustHostKey {
        host: String,
        port: u16,
        fingerprint: String,
    },
    PrivateKeyPassphrase {
        private_key_path: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InteractionReplyDto {
    TrustHostKey { trusted: bool },
    PrivateKeyPassphrase { passphrase: String },
}

struct DesktopState {
    runtime: Mutex<RuntimeManager>,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            runtime: Mutex::new(RuntimeManager::new()),
        }
    }
}

#[tauri::command]
fn load_config() -> Result<LoadConfigResponse, String> {
    let path = default_config_path();
    let exists = path.exists();
    let config = if exists {
        ConfigDraft::from_client_config(ClientConfig::load(None).map_err(|err| err.to_string())?)
    } else {
        ConfigDraft::default()
    };

    Ok(LoadConfigResponse {
        config,
        path: path.to_string_lossy().into_owned(),
        exists,
    })
}

#[tauri::command]
fn save_config(config: ConfigDraft) -> Result<SaveConfigResponse, String> {
    let config = config.try_into_client_config()?;
    let path = config.save(None).map_err(|err| err.to_string())?;
    Ok(SaveConfigResponse {
        path: path.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
fn check_connection(
    config: ConfigDraft,
    interaction: Option<InteractionReplyDto>,
) -> Result<CommandResponse, String> {
    let config = config.try_into_client_config()?;
    let interaction = interaction.map(InteractionReplyDto::into_interaction_response);
    map_client_action(
        run_check(config, interaction).map_err(|err| err.to_string())?,
        "Connection check passed.",
    )
}

#[tauri::command]
fn start_runtime(
    state: tauri::State<'_, DesktopState>,
    config: ConfigDraft,
    interaction: Option<InteractionReplyDto>,
) -> Result<CommandResponse, String> {
    let config = config.try_into_client_config()?;
    let interaction = interaction.map(InteractionReplyDto::into_interaction_response);
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime manager lock was poisoned".to_string())?;

    match runtime
        .start(config, interaction)
        .map_err(|err| err.to_string())?
    {
        ClientActionState::Ready(status) => Ok(CommandResponse::Ok {
            message: "Upload runtime started.".to_string(),
            runtime_status: Some(runtime_status_label(status)),
        }),
        ClientActionState::NeedsInteraction(request) => Ok(CommandResponse::NeedsInteraction {
            request: InteractionRequestDto::from_request(request),
        }),
    }
}

#[tauri::command]
fn stop_runtime(state: tauri::State<'_, DesktopState>) -> Result<RuntimeStatusResponse, String> {
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime manager lock was poisoned".to_string())?;
    let status = runtime.stop().map_err(|err| err.to_string())?;
    Ok(RuntimeStatusResponse {
        status: runtime_status_label(status),
    })
}

#[tauri::command]
fn runtime_status(state: tauri::State<'_, DesktopState>) -> Result<RuntimeStatusResponse, String> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime manager lock was poisoned".to_string())?;
    Ok(RuntimeStatusResponse {
        status: runtime_status_label(runtime.status()),
    })
}

impl ConfigDraft {
    fn from_client_config(config: ClientConfig) -> Self {
        Self {
            user_name: config.user_name,
            hotkey: config.hotkey,
            upload: UploadDraft {
                host: config.upload.host,
                port: config.upload.port,
                user: config.upload.user,
                private_key_path: config
                    .upload
                    .private_key_path
                    .to_string_lossy()
                    .into_owned(),
                shared_image_root: config.upload.shared_image_root,
            },
        }
    }

    fn try_into_client_config(self) -> Result<ClientConfig, String> {
        let config = ClientConfig {
            user_name: self.user_name,
            hotkey: self.hotkey,
            upload: UploadConfig {
                host: self.upload.host,
                port: self.upload.port,
                user: self.upload.user,
                private_key_path: PathBuf::from(self.upload.private_key_path),
                shared_image_root: self.upload.shared_image_root,
            },
        };

        config.validate().map_err(|err| err.to_string())?;
        Ok(config)
    }
}

impl Default for ConfigDraft {
    fn default() -> Self {
        Self {
            user_name: String::new(),
            hotkey: "Ctrl+Alt+U".to_string(),
            upload: UploadDraft {
                host: String::new(),
                port: 22,
                user: String::new(),
                private_key_path: String::new(),
                shared_image_root: String::new(),
            },
        }
    }
}

impl InteractionRequestDto {
    fn from_request(request: InteractionRequest) -> Self {
        match request {
            InteractionRequest::TrustHostKey(request) => Self::TrustHostKey {
                host: request.host,
                port: request.port,
                fingerprint: request.fingerprint,
            },
            InteractionRequest::PrivateKeyPassphrase(request) => Self::PrivateKeyPassphrase {
                private_key_path: request.private_key_path.to_string_lossy().into_owned(),
            },
        }
    }
}

impl InteractionReplyDto {
    fn into_interaction_response(self) -> InteractionResponse {
        match self {
            Self::TrustHostKey { trusted } => InteractionResponse::TrustHostKey(trusted),
            Self::PrivateKeyPassphrase { passphrase } => {
                InteractionResponse::PrivateKeyPassphrase(passphrase)
            }
        }
    }
}

fn map_client_action(
    action: ClientActionState<()>,
    success_message: &str,
) -> Result<CommandResponse, String> {
    match action {
        ClientActionState::Ready(()) => Ok(CommandResponse::Ok {
            message: success_message.to_string(),
            runtime_status: None,
        }),
        ClientActionState::NeedsInteraction(request) => Ok(CommandResponse::NeedsInteraction {
            request: InteractionRequestDto::from_request(request),
        }),
    }
}

fn runtime_status_label(status: RuntimeStatus) -> &'static str {
    match status {
        RuntimeStatus::Running => "running",
        RuntimeStatus::Stopped => "stopped",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState::default())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            check_connection,
            start_runtime,
            stop_runtime,
            runtime_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
