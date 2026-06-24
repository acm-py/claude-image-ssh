use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionRequest {
    TrustHostKey(TrustHostKeyRequest),
    PrivateKeyPassphrase(PrivateKeyPassphraseRequest),
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
pub enum InteractionResponse {
    TrustHostKey(bool),
    PrivateKeyPassphrase(String),
}

impl InteractionResponse {
    pub fn expect_trust_host_key(self) -> Option<bool> {
        match self {
            Self::TrustHostKey(value) => Some(value),
            Self::PrivateKeyPassphrase(_) => None,
        }
    }

    pub fn expect_private_key_passphrase(self) -> Option<String> {
        match self {
            Self::PrivateKeyPassphrase(value) => Some(value),
            Self::TrustHostKey(_) => None,
        }
    }
}
