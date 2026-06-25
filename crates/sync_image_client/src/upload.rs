use std::{
    fs,
    io::Write,
    net::TcpStream,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use sha2::{Digest, Sha256};
use ssh2::{Session, Sftp};
use sync_image_core::{AuthMethod, ClientConfig, build_user_dir, format_upload_filename};
use uuid::Uuid;

use crate::interaction::{
    InteractionRequest, InteractionResponse, PasswordRequest, PrivateKeyPassphraseRequest,
    TrustHostKeyRequest,
};

const CHECK_FILE_NAME: &str = ".claude-image-sync-check.tmp";

#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub file_name: String,
}

pub struct SftpUploader {
    config: ClientConfig,
    session: Session,
    /// Password captured through interaction during this connection that is not
    /// yet persisted in the config file. Taken by the caller to write it back.
    newly_captured_password: Option<String>,
}

#[derive(Debug)]
pub enum ConnectError {
    NeedsInteraction(InteractionRequest),
    Failed(anyhow::Error),
}

impl From<anyhow::Error> for ConnectError {
    fn from(value: anyhow::Error) -> Self {
        Self::Failed(value)
    }
}

impl SftpUploader {
    pub fn connect(
        mut config: ClientConfig,
        response: Option<InteractionResponse>,
    ) -> std::result::Result<Self, ConnectError> {
        let session = connect_session(&config, response.clone())?;

        // A password supplied interactively (not already in the config) is kept
        // in memory so reconnects can reuse it, and exposed for write-back.
        let newly_captured_password = match config.upload.auth_method {
            AuthMethod::Password if config.upload.password.is_none() => {
                response.and_then(InteractionResponse::expect_password)
            }
            _ => None,
        };
        if let Some(password) = &newly_captured_password {
            config.upload.password = Some(password.clone());
        }

        Ok(Self {
            config,
            session,
            newly_captured_password,
        })
    }

    /// Takes the password captured interactively during connect, if any, so the
    /// caller can persist it to the config file after a successful connection.
    pub fn take_newly_captured_password(&mut self) -> Option<String> {
        self.newly_captured_password.take()
    }

    /// Persists the config (including a freshly captured password) to disk when a
    /// password was supplied interactively during connect. No-op otherwise.
    pub fn persist_captured_password(&mut self, config_path: Option<PathBuf>) -> Result<()> {
        if self.take_newly_captured_password().is_some() {
            self.config
                .save(config_path)
                .context("failed to persist password to config file")?;
        }
        Ok(())
    }

    pub fn upload_png(&mut self, png_bytes: &[u8]) -> Result<UploadedFile> {
        match self.upload_png_once(png_bytes) {
            Ok(uploaded) => Ok(uploaded),
            Err(first_error) => {
                eprintln!("upload failed, reconnecting once: {first_error}");
                self.reconnect()?;
                self.upload_png_once(png_bytes)
            }
        }
    }

    pub fn check_remote_write_access(&mut self) -> Result<()> {
        let sftp = self.session.sftp().context("failed to open SFTP session")?;
        let user_dir = self.ensure_user_dir(&sftp)?;
        let probe = user_dir.join(CHECK_FILE_NAME);

        {
            let mut file = sftp
                .create(&probe)
                .with_context(|| format!("failed to create {}", probe.display()))?;
            file.write_all(b"claude-image-sync check\n")?;
        }

        sftp.unlink(&probe)
            .with_context(|| format!("failed to delete {}", probe.display()))?;
        Ok(())
    }

    fn reconnect(&mut self) -> Result<()> {
        self.session = connect_session(&self.config, None).map_err(|err| match err {
            ConnectError::NeedsInteraction(request) => {
                anyhow::anyhow!("reconnect requires interaction: {request:?}")
            }
            ConnectError::Failed(err) => err,
        })?;
        Ok(())
    }

    fn upload_png_once(&self, png_bytes: &[u8]) -> Result<UploadedFile> {
        let sftp = self.session.sftp().context("failed to open SFTP session")?;
        let user_dir = self.ensure_user_dir(&sftp)?;
        let file_name = format_upload_filename(Utc::now());
        let final_path = user_dir.join(&file_name);
        let temp_path = user_dir.join(format!(".{}.uploading", Uuid::new_v4()));

        {
            let mut file = sftp
                .create(&temp_path)
                .with_context(|| format!("failed to create {}", temp_path.display()))?;
            file.write_all(png_bytes)
                .with_context(|| format!("failed to write {}", temp_path.display()))?;
        }

        sftp.rename(&temp_path, &final_path, None)
            .with_context(|| format!("failed to rename {}", final_path.display()))?;

        Ok(UploadedFile { file_name })
    }

    fn ensure_user_dir(&self, sftp: &Sftp) -> Result<PathBuf> {
        let user_dir = build_user_dir(
            Path::new(&self.config.upload.shared_image_root),
            &self.config.user_name,
        )?;
        ensure_remote_dir(sftp, &user_dir)?;
        Ok(user_dir)
    }
}

fn connect_session(
    config: &ClientConfig,
    response: Option<InteractionResponse>,
) -> std::result::Result<Session, ConnectError> {
    let address = format!("{}:{}", config.upload.host, config.upload.port);
    let tcp =
        TcpStream::connect(&address).with_context(|| format!("failed to connect {address}"))?;
    let mut session = Session::new().context("failed to create SSH session")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("SSH handshake failed")?;
    verify_host_key(config, &session, response.clone())?;
    authenticate(config, &mut session, response)?;
    Ok(session)
}

fn verify_host_key(
    config: &ClientConfig,
    session: &Session,
    response: Option<InteractionResponse>,
) -> std::result::Result<(), ConnectError> {
    let (host_key, _) = session
        .host_key()
        .context("SSH server did not provide a host key")?;
    let fingerprint = sha256_hex(host_key);
    let cache_path = host_key_cache_path(config)?;

    if cache_path.exists() {
        let expected = fs::read_to_string(&cache_path)
            .with_context(|| format!("failed to read {}", cache_path.display()))?;
        if expected.trim() != fingerprint {
            return Err(anyhow!(
                "upload host key changed for {}:{}; expected {}, got {}",
                config.upload.host,
                config.upload.port,
                expected.trim(),
                fingerprint
            )
            .into());
        }
        return Ok(());
    }

    let trusted = match response.and_then(|value| value.expect_trust_host_key()) {
        Some(value) => value,
        None => {
            return Err(ConnectError::NeedsInteraction(
                InteractionRequest::TrustHostKey(TrustHostKeyRequest {
                    host: config.upload.host.clone(),
                    port: config.upload.port,
                    fingerprint,
                }),
            ));
        }
    };

    if !trusted {
        return Err(anyhow!("host key was not trusted").into());
    }

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let fingerprint = session
        .host_key()
        .context("SSH server did not provide a host key")?
        .0;
    fs::write(&cache_path, format!("{}\n", sha256_hex(fingerprint)))
        .with_context(|| format!("failed to write {}", cache_path.display()))?;
    Ok(())
}

fn authenticate(
    config: &ClientConfig,
    session: &mut Session,
    response: Option<InteractionResponse>,
) -> std::result::Result<(), ConnectError> {
    match config.upload.auth_method {
        AuthMethod::Key => authenticate_with_key(config, session, response),
        AuthMethod::Password => authenticate_with_password(config, session, response),
    }
}

fn authenticate_with_password(
    config: &ClientConfig,
    session: &mut Session,
    response: Option<InteractionResponse>,
) -> std::result::Result<(), ConnectError> {
    let password = match &config.upload.password {
        Some(password) => password.clone(),
        None => match response.and_then(InteractionResponse::expect_password) {
            Some(password) => password,
            None => {
                return Err(ConnectError::NeedsInteraction(InteractionRequest::Password(
                    PasswordRequest {
                        host: config.upload.host.clone(),
                        port: config.upload.port,
                        user: config.upload.user.clone(),
                    },
                )));
            }
        },
    };

    session
        .userauth_password(&config.upload.user, &password)
        .context("password authentication failed")?;

    if !session.authenticated() {
        return Err(anyhow!("password authentication did not complete").into());
    }

    Ok(())
}

fn authenticate_with_key(
    config: &ClientConfig,
    session: &mut Session,
    response: Option<InteractionResponse>,
) -> std::result::Result<(), ConnectError> {
    let key_path = &config.upload.private_key_path;
    match session.userauth_pubkey_file(&config.upload.user, None, key_path, None) {
        Ok(()) if session.authenticated() => return Ok(()),
        Ok(()) => return Err(anyhow!("private key authentication did not complete").into()),
        Err(first_error) => {
            eprintln!("private key auth without passphrase failed: {first_error}");
        }
    }

    let passphrase = match response.and_then(|value| value.expect_private_key_passphrase()) {
        Some(value) => value,
        None => {
            return Err(ConnectError::NeedsInteraction(
                InteractionRequest::PrivateKeyPassphrase(PrivateKeyPassphraseRequest {
                    private_key_path: key_path.clone(),
                }),
            ));
        }
    };

    session
        .userauth_pubkey_file(&config.upload.user, None, key_path, Some(&passphrase))
        .context("private key authentication failed")?;

    if !session.authenticated() {
        return Err(anyhow!("private key authentication did not complete").into());
    }

    Ok(())
}

fn ensure_remote_dir(sftp: &Sftp, path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        if current.as_os_str().is_empty() || current == Path::new("/") {
            continue;
        }
        if sftp.stat(&current).is_ok() {
            continue;
        }
        sftp.mkdir(&current, 0o755)
            .with_context(|| format!("failed to create {}", current.display()))?;
    }
    Ok(())
}

fn host_key_cache_path(config: &ClientConfig) -> Result<PathBuf> {
    let mut dir = sync_image_core::config::default_config_path();
    dir.pop();
    let safe_host = config
        .upload
        .host
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(dir
        .join("known-hosts")
        .join(format!("{}_{}.sha256", safe_host, config.upload.port)))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}
