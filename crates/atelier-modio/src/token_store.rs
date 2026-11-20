use std::sync::Mutex;

use crate::error::{Error, Result};

const KEYRING_SERVICE: &str = "anno-mod-manager";
const TOKEN_USER: &str = "modio-token";
const API_KEY_USER: &str = "modio-api-key";

/// Where the mod.io access token lives between runs.
pub trait TokenStore: Send + Sync {
    fn load(&self) -> Result<Option<String>>;
    fn save(&self, token: &str) -> Result<()>;
    fn clear(&self) -> Result<()>;
}

/// OS credential store (Windows Credential Manager, macOS Keychain, Secret Service).
#[derive(Debug)]
pub struct KeyringTokenStore {
    user: &'static str,
}

impl Default for KeyringTokenStore {
    /// The signed-in user's access token.
    fn default() -> Self {
        Self { user: TOKEN_USER }
    }
}

impl KeyringTokenStore {
    /// The user's own mod.io API key, kept apart from the access token.
    pub fn api_key() -> Self {
        Self { user: API_KEY_USER }
    }
}

// Keyring errors never contain the secret itself, so their text is safe to surface.
fn store_error(error: keyring::Error) -> Error {
    Error::TokenStore(error.to_string())
}

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<String>> {
        load_split(&KeyringSlots, self.user)
    }

    fn save(&self, token: &str) -> Result<()> {
        save_split(&KeyringSlots, self.user, token)
    }

    fn clear(&self) -> Result<()> {
        clear_split(&KeyringSlots, self.user)
    }
}

/// Named secrets in one store; lets the splitting below be tested without the OS.
trait Slots {
    fn get(&self, name: &str) -> Result<Option<String>>;
    fn set(&self, name: &str, value: &str) -> Result<()>;
    fn delete(&self, name: &str) -> Result<()>;
}

struct KeyringSlots;

impl KeyringSlots {
    fn entry(name: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, name).map_err(store_error)
    }
}

impl Slots for KeyringSlots {
    fn get(&self, name: &str) -> Result<Option<String>> {
        match Self::entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(store_error(error)),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<()> {
        Self::entry(name)?.set_password(value).map_err(store_error)
    }

    fn delete(&self, name: &str) -> Result<()> {
        match Self::entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(store_error(error)),
        }
    }
}

/// Windows caps one credential at 2560 bytes of UTF-16 (1280 characters), and a mod.io
/// access token is a longer JWT, so long secrets are stored in parts.
const PART_CHARS: usize = 1000;
/// Marks a header entry that names how many parts follow; no token or key starts with it.
const PARTS_HEADER: &str = "atelier-parts:";

/// How many parts a header entry names; `None` for a plain secret.
fn part_count(head: &str) -> Option<usize> {
    head.strip_prefix(PARTS_HEADER)?.parse().ok()
}

fn part_name(user: &str, index: usize) -> String {
    format!("{user}.{index}")
}

fn load_split(slots: &dyn Slots, user: &str) -> Result<Option<String>> {
    let Some(head) = slots.get(user)? else {
        return Ok(None);
    };
    if !head.starts_with(PARTS_HEADER) {
        return Ok(Some(head));
    }
    let count = part_count(&head)
        .ok_or_else(|| Error::TokenStore("unreadable saved sign-in; sign in again".into()))?;
    let mut secret = String::new();
    for index in 0..count {
        let part = slots.get(&part_name(user, index))?.ok_or_else(|| {
            Error::TokenStore("part of the saved sign-in is missing; sign in again".into())
        })?;
        secret.push_str(&part);
    }
    Ok(Some(secret))
}

/// Parts go first and the header last, so an interrupted save never reads as complete.
fn save_split(slots: &dyn Slots, user: &str, secret: &str) -> Result<()> {
    clear_split(slots, user)?;
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= PART_CHARS {
        return slots.set(user, secret);
    }
    let parts: Vec<String> = chars
        .chunks(PART_CHARS)
        .map(|part| part.iter().collect())
        .collect();
    for (index, part) in parts.iter().enumerate() {
        slots.set(&part_name(user, index), part)?;
    }
    slots.set(user, &format!("{PARTS_HEADER}{}", parts.len()))
}

fn clear_split(slots: &dyn Slots, user: &str) -> Result<()> {
    if let Some(count) = slots.get(user)?.as_deref().and_then(part_count) {
        for index in 0..count {
            slots.delete(&part_name(user, index))?;
        }
    }
    slots.delete(user)
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Holds secrets the way Windows does: at most 1280 characters each.
    #[derive(Default)]
    struct WindowsLikeSlots(Mutex<HashMap<String, String>>);

    impl Slots for WindowsLikeSlots {
        fn get(&self, name: &str) -> Result<Option<String>> {
            Ok(self.0.lock().unwrap().get(name).cloned())
        }
        fn set(&self, name: &str, value: &str) -> Result<()> {
            if value.chars().count() > 1280 {
                return Err(Error::TokenStore("longer than platform limit".into()));
            }
            self.0.lock().unwrap().insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<()> {
            self.0.lock().unwrap().remove(name);
            Ok(())
        }
    }

    #[test]
    fn a_token_longer_than_one_credential_round_trips() {
        let slots = WindowsLikeSlots::default();
        let token = "x".repeat(3000) + "end";

        save_split(&slots, "modio-token", &token).unwrap();

        assert_eq!(load_split(&slots, "modio-token").unwrap(), Some(token));
    }

    #[test]
    fn clearing_removes_every_part() {
        let slots = WindowsLikeSlots::default();
        save_split(&slots, "modio-token", &"y".repeat(2500)).unwrap();

        clear_split(&slots, "modio-token").unwrap();

        assert!(slots.0.lock().unwrap().is_empty());
        assert_eq!(load_split(&slots, "modio-token").unwrap(), None);
    }

    #[test]
    fn a_shorter_token_leaves_no_parts_of_a_longer_one_behind() {
        let slots = WindowsLikeSlots::default();
        save_split(&slots, "modio-token", &"z".repeat(3000)).unwrap();

        save_split(&slots, "modio-token", "short-key").unwrap();

        assert_eq!(slots.0.lock().unwrap().len(), 1);
        assert_eq!(
            load_split(&slots, "modio-token").unwrap().as_deref(),
            Some("short-key")
        );
    }

    #[test]
    fn a_secret_saved_by_an_earlier_version_still_loads() {
        let slots = WindowsLikeSlots::default();
        slots.set("modio-api-key", "0123456789abcdef").unwrap();

        assert_eq!(
            load_split(&slots, "modio-api-key").unwrap().as_deref(),
            Some("0123456789abcdef")
        );
    }
}
