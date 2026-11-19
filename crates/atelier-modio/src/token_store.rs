use std::sync::Mutex;

use crate::error::{Error, Result};

const KEYRING_SERVICE: &str = "anno-mod-manager";
const KEYRING_USER: &str = "modio-token";

/// Where the mod.io access token lives between runs.
pub trait TokenStore: Send + Sync {
    fn load(&self) -> Result<Option<String>>;
    fn save(&self, token: &str) -> Result<()>;
    fn clear(&self) -> Result<()>;
}

/// OS credential store (Windows Credential Manager, macOS Keychain, Secret Service).
#[derive(Debug, Default)]
pub struct KeyringTokenStore;

impl KeyringTokenStore {
    fn entry() -> Result<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(store_error)
    }
}

// Keyring errors never contain the secret itself, so their text is safe to surface.
fn store_error(error: keyring::Error) -> Error {
    Error::TokenStore(error.to_string())
}

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<String>> {
        match Self::entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(store_error(error)),
        }
    }

    fn save(&self, token: &str) -> Result<()> {
        Self::entry()?.set_password(token).map_err(store_error)
    }

    fn clear(&self) -> Result<()> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(store_error(error)),
        }
    }
}

/// Process-local store for tests and headless runs.
#[derive(Default)]
pub struct MemoryTokenStore {
    token: Mutex<Option<String>>,
}

impl MemoryTokenStore {
    pub fn with_token(token: &str) -> Self {
        Self {
            token: Mutex::new(Some(token.to_owned())),
        }
    }

    fn slot(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        // A poisoned lock still holds a valid Option; recover it.
        self.token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl std::fmt::Debug for MemoryTokenStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("MemoryTokenStore { .. }")
    }
}

impl TokenStore for MemoryTokenStore {
    fn load(&self) -> Result<Option<String>> {
        Ok(self.slot().clone())
    }

    fn save(&self, token: &str) -> Result<()> {
        *self.slot() = Some(token.to_owned());
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        *self.slot() = None;
        Ok(())
    }
}
