use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionRequest {
    TrustHostKey(TrustHostKeyRequest),
    PrivateKeyPassphrase(PrivateKeyPassphraseRequest),
    Password(PasswordRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustHostKeyRequest {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKeyPassphraseRequest {
    pub private_key_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordRequest {
    pub host: String,
    pub port: u16,
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionResponse {
    TrustHostKey(bool),
    PrivateKeyPassphrase(String),
    Password(String),
}

impl InteractionResponse {
    pub fn expect_trust_host_key(self) -> Option<bool> {
        match self {
            Self::TrustHostKey(value) => Some(value),
            Self::PrivateKeyPassphrase(_) | Self::Password(_) => None,
        }
    }

    pub fn expect_private_key_passphrase(self) -> Option<String> {
        match self {
            Self::PrivateKeyPassphrase(value) => Some(value),
            Self::TrustHostKey(_) | Self::Password(_) => None,
        }
    }

    pub fn expect_password(self) -> Option<String> {
        match self {
            Self::Password(value) => Some(value),
            Self::TrustHostKey(_) | Self::PrivateKeyPassphrase(_) => None,
        }
    }
}
