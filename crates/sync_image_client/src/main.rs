use anyhow::Result;
use clap::Parser;
use sync_image_client::{
    cli::{Cli, Command},
    client::{ClientActionState, run_check, run_client_once},
    interaction::{InteractionRequest, InteractionResponse},
};
use sync_image_core::ClientConfig;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.clone();
    let config = ClientConfig::load(config_path.clone())?;

    match cli.command.unwrap_or(Command::Run) {
        Command::Run => run_client(config, config_path),
        Command::Check => run_check_command(config, config_path),
    }
}

fn run_client(config: ClientConfig, config_path: Option<std::path::PathBuf>) -> Result<()> {
    let mut response = None;
    loop {
        match run_client_once(config.clone(), response.take(), config_path.clone())? {
            ClientActionState::Ready(()) => return Ok(()),
            ClientActionState::NeedsInteraction(request) => {
                response = Some(prompt_for_interaction(request)?);
            }
        }
    }
}

fn run_check_command(config: ClientConfig, config_path: Option<std::path::PathBuf>) -> Result<()> {
    let mut response = None;
    loop {
        match run_check(config.clone(), response.take(), config_path.clone())? {
            ClientActionState::Ready(()) => {
                println!(
                    "check passed: configuration, SSH/SFTP, host key and remote write access are valid"
                );
                return Ok(());
            }
            ClientActionState::NeedsInteraction(request) => {
                response = Some(prompt_for_interaction(request)?);
            }
        }
    }
}

fn prompt_for_interaction(request: InteractionRequest) -> Result<InteractionResponse> {
    match request {
        InteractionRequest::TrustHostKey(prompt) => {
            println!(
                "First connection to {}:{} host key SHA256 fingerprint:\n{}",
                prompt.host, prompt.port, prompt.fingerprint
            );
            use std::io::{self, Write};
            print!("Trust this host key? [y/N] ");
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            Ok(InteractionResponse::TrustHostKey(matches!(
                answer.trim(),
                "y" | "Y" | "yes" | "YES"
            )))
        }
        InteractionRequest::PrivateKeyPassphrase(prompt) => {
            let message = format!(
                "Private key passphrase for {}: ",
                prompt.private_key_path.display()
            );
            let passphrase = rpassword::prompt_password(message)?;
            Ok(InteractionResponse::PrivateKeyPassphrase(passphrase))
        }
        InteractionRequest::Password(prompt) => {
            let message = format!(
                "SSH password for {}@{}:{}: ",
                prompt.user, prompt.host, prompt.port
            );
            let password = rpassword::prompt_password(message)?;
            Ok(InteractionResponse::Password(password))
        }
    }
}
