use std::{
    fs,
    io::{self, Write},
    net::TcpStream,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use sha2::{Digest, Sha256};
use ssh2::{Session, Sftp};
use sync_image_core::{ClientConfig, build_user_dir, format_upload_filename};
use uuid::Uuid;

const CHECK_FILE_NAME: &str = ".claude-image-sync-check.tmp";

#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub file_name: String,
}

pub struct SftpUploader {
    config: ClientConfig,
    session: Session,
}

impl SftpUploader {
    pub fn connect(config: ClientConfig) -> Result<Self> {
        let session = connect_session(&config)?;
        Ok(Self { config, session })
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
        self.session = connect_session(&self.config)?;
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

fn connect_session(config: &ClientConfig) -> Result<Session> {
    let address = format!("{}:{}", config.upload.host, config.upload.port);
    let tcp =
        TcpStream::connect(&address).with_context(|| format!("failed to connect {address}"))?;
    let mut session = Session::new().context("failed to create SSH session")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("SSH handshake failed")?;
    verify_host_key(config, &session)?;
    authenticate(config, &mut session)?;
    Ok(session)
}

fn verify_host_key(config: &ClientConfig, session: &Session) -> Result<()> {
    let (host_key, _) = session
        .host_key()
        .context("SSH server did not provide a host key")?;
    let fingerprint = sha256_hex(host_key);
    let cache_path = host_key_cache_path(config)?;

    if cache_path.exists() {
        let expected = fs::read_to_string(&cache_path)
            .with_context(|| format!("failed to read {}", cache_path.display()))?;
        if expected.trim() != fingerprint {
            bail!(
                "upload host key changed for {}:{}; expected {}, got {}",
                config.upload.host,
                config.upload.port,
                expected.trim(),
                fingerprint
            );
        }
        return Ok(());
    }

    println!(
        "First connection to {}:{} host key SHA256 fingerprint:\n{}",
        config.upload.host, config.upload.port, fingerprint
    );
    print!("Trust this host key? [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
        bail!("host key was not trusted");
    }

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&cache_path, format!("{fingerprint}\n"))
        .with_context(|| format!("failed to write {}", cache_path.display()))?;
    Ok(())
}

fn authenticate(config: &ClientConfig, session: &mut Session) -> Result<()> {
    let key_path = &config.upload.private_key_path;
    match session.userauth_pubkey_file(&config.upload.user, None, key_path, None) {
        Ok(()) if session.authenticated() => return Ok(()),
        Ok(()) => bail!("private key authentication did not complete"),
        Err(first_error) => {
            eprintln!("private key auth without passphrase failed: {first_error}");
        }
    }

    let passphrase = rpassword::prompt_password("Private key passphrase: ")?;
    session
        .userauth_pubkey_file(&config.upload.user, None, key_path, Some(&passphrase))
        .context("private key authentication failed")?;

    if !session.authenticated() {
        bail!("private key authentication did not complete");
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
