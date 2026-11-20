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

/// A split secret spans several entries, so one read, save or clear runs at a time.
static SPLIT_LOCK: Mutex<()> = Mutex::new(());

fn split_lock() -> std::sync::MutexGuard<'static, ()> {
    SPLIT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<String>> {
        let _held = split_lock();
        load_split(&KeyringSlots, self.user)
    }

    fn save(&self, token: &str) -> Result<()> {
        let _held = split_lock();
        save_split(&KeyringSlots, self.user, token)
    }

    fn clear(&self) -> Result<()> {
        let _held = split_lock();
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
/// A header entry reads `atelier-parts:<set>:<count>`; no token or key starts with it.
const PARTS_HEADER: &str = "atelier-parts:";

/// Where a split secret's parts are. Two sets alternate, so the next save writes beside
/// the current parts and a failed save leaves them intact.
#[derive(Clone, Copy)]
struct Parts<'a> {
    set: &'a str,
    count: usize,
}

impl<'a> Parts<'a> {
    fn read(head: &'a str) -> Option<Self> {
        let (set, count) = head.strip_prefix(PARTS_HEADER)?.split_once(':')?;
        Some(Self {
            set,
            count: count.parse().ok()?,
        })
    }

    fn name(self, user: &str, index: usize) -> String {
        format!("{user}.{}.{index}", self.set)
    }
}

fn load_split(slots: &dyn Slots, user: &str) -> Result<Option<String>> {
    let Some(head) = slots.get(user)? else {
        return Ok(None);
    };
    if !head.starts_with(PARTS_HEADER) {
        return Ok(Some(head));
    }
    let parts = Parts::read(&head)
        .ok_or_else(|| Error::TokenStore("unreadable saved sign-in; sign in again".into()))?;
    let mut secret = String::new();
    for index in 0..parts.count {
        // A part gone after an interrupted clear means no usable sign-in: signed out, with
        // the header left for the next save or clear to replace.
        let Some(part) = slots.get(&parts.name(user, index))? else {
            return Ok(None);
        };
        secret.push_str(&part);
    }
    Ok(Some(secret))
}

/// The new secret is complete before the header points at it; only then do the old parts go.
fn save_split(slots: &dyn Slots, user: &str, secret: &str) -> Result<()> {
    let old_head = slots.get(user)?;
    let old = old_head.as_deref().and_then(Parts::read);
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= PART_CHARS {
        slots.set(user, secret)?;
    } else {
        let set = if old.is_some_and(|old| old.set == "a") {
            "b"
        } else {
            "a"
        };
        let chunks: Vec<String> = chars
            .chunks(PART_CHARS)
            .map(|chunk| chunk.iter().collect())
            .collect();
        let parts = Parts {
            set,
            count: chunks.len(),
        };
        let written = chunks
            .iter()
            .enumerate()
            .try_for_each(|(index, chunk)| slots.set(&parts.name(user, index), chunk))
            .and_then(|()| slots.set(user, &format!("{PARTS_HEADER}{set}:{}", parts.count)));
        if let Err(error) = written {
            // The old sign-in is untouched; the half-written new one must not linger.
            for index in 0..parts.count {
                let _ = slots.delete(&parts.name(user, index));
            }
            return Err(error);
        }
    }
    // The new secret is saved; a leftover old part is harmless, so this can't fail the save.
    if let Some(old) = old {
        for index in 0..old.count {
            let _ = slots.delete(&old.name(user, index));
        }
    }
    Ok(())
}

fn clear_split(slots: &dyn Slots, user: &str) -> Result<()> {
    // Parts first and the header last: a clear that fails partway still names what is left,
    // so running it again finishes the job. The store lock keeps readers out meanwhile.
    if let Some(parts) = slots.get(user)?.as_deref().and_then(Parts::read) {
        for index in 0..parts.count {
            slots.delete(&parts.name(user, index))?;
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

    use std::sync::atomic::{AtomicBool, Ordering};

    /// Holds secrets the way Windows does: at most 1280 characters each.
    #[derive(Default)]
    struct WindowsLikeSlots {
        entries: Mutex<HashMap<String, String>>,
        /// Fails every write, like a full or locked store.
        fail_writes: AtomicBool,
        /// Fails only the header write, after the parts are already in.
        fail_header: AtomicBool,
        /// Fails the next delete, like a credential Windows briefly holds open.
        fail_next_delete: AtomicBool,
    }

    impl WindowsLikeSlots {
        fn len(&self) -> usize {
            self.entries.lock().unwrap().len()
        }
    }

    impl Slots for WindowsLikeSlots {
        fn get(&self, name: &str) -> Result<Option<String>> {
            Ok(self.entries.lock().unwrap().get(name).cloned())
        }
        fn set(&self, name: &str, value: &str) -> Result<()> {
            let header = !name.contains('.');
            if self.fail_writes.load(Ordering::SeqCst)
                || (header && self.fail_header.load(Ordering::SeqCst))
            {
                return Err(Error::TokenStore("disk full".into()));
            }
            if value.chars().count() > 1280 {
                return Err(Error::TokenStore("longer than platform limit".into()));
            }
            self.entries
                .lock()
                .unwrap()
                .insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<()> {
            if self.fail_next_delete.swap(false, Ordering::SeqCst) {
                return Err(Error::TokenStore("in use".into()));
            }
            self.entries.lock().unwrap().remove(name);
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

        assert_eq!(slots.len(), 0);
        assert_eq!(load_split(&slots, "modio-token").unwrap(), None);
    }

    #[test]
    fn a_shorter_token_leaves_no_parts_of_a_longer_one_behind() {
        let slots = WindowsLikeSlots::default();
        save_split(&slots, "modio-token", &"z".repeat(3000)).unwrap();

        save_split(&slots, "modio-token", "short-key").unwrap();

        assert_eq!(slots.len(), 1);
        assert_eq!(
            load_split(&slots, "modio-token").unwrap().as_deref(),
            Some("short-key")
        );
    }

    #[test]
    fn a_failed_replacement_keeps_the_working_sign_in() {
        let slots = WindowsLikeSlots::default();
        let working = "w".repeat(2500);
        save_split(&slots, "modio-token", &working).unwrap();
        slots.fail_writes.store(true, Ordering::SeqCst);

        assert!(save_split(&slots, "modio-token", &"n".repeat(2500)).is_err());

        assert_eq!(load_split(&slots, "modio-token").unwrap(), Some(working));
    }

    #[test]
    fn a_clear_that_fails_partway_finishes_when_run_again() {
        let slots = WindowsLikeSlots::default();
        save_split(&slots, "modio-token", &"c".repeat(2500)).unwrap();
        slots.fail_next_delete.store(true, Ordering::SeqCst);

        assert!(clear_split(&slots, "modio-token").is_err());
        clear_split(&slots, "modio-token").unwrap();

        assert_eq!(slots.len(), 0);
    }

    #[test]
    fn a_save_that_fails_at_the_header_leaves_no_new_parts() {
        let slots = WindowsLikeSlots::default();
        slots.fail_header.store(true, Ordering::SeqCst);

        assert!(save_split(&slots, "modio-token", &"s".repeat(2500)).is_err());

        assert_eq!(slots.len(), 0);
    }

    #[test]
    fn a_sign_in_missing_a_part_reads_as_signed_out() {
        let slots = WindowsLikeSlots::default();
        save_split(&slots, "modio-token", &"m".repeat(2500)).unwrap();
        slots.delete("modio-token.a.1").unwrap();

        assert_eq!(load_split(&slots, "modio-token").unwrap(), None);
        clear_split(&slots, "modio-token").unwrap();
        assert_eq!(slots.len(), 0);
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
