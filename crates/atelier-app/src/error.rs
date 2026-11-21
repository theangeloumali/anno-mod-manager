//! The one error shape the UI sees: a stable code plus a sentence it can show as is.

use std::fmt;

use serde::Serialize;

pub(crate) const OFFLINE_MESSAGE: &str = "mod.io is unreachable. Check your connection.";
pub(crate) const NO_API_KEY_MESSAGE: &str = "Add your mod.io API key in Settings to browse mod.io.";
pub(crate) const INVALID_KEY_MESSAGE: &str =
    "mod.io doesn't recognise that key. Copy it again from mod.io/me/access.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    NotSupported,
    GameRunning,
    GameNotFound,
    NoApiKey,
    InvalidApiKey,
    Offline,
    Unauthorized,
    NotFound,
    NotSignedIn,
    TermsRequired,
    RateLimited,
    ReadOnly,
    InvalidInput,
    Io,
    Install,
    Modio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn io(error: &std::io::Error, path: &std::path::Path) -> Self {
        Self::new(ErrorCode::Io, format!("{}: {error}", path.display()))
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<atelier_core::Error> for AppError {
    fn from(error: atelier_core::Error) -> Self {
        use atelier_core::Error as Core;
        let code = match &error {
            Core::ReadOnly(_) => ErrorCode::ReadOnly,
            Core::ProfileNotFound(_) | Core::RegistryNotFound(_) | Core::MissingModules(_) => {
                ErrorCode::NotFound
            }
            Core::BuiltInProfile(_)
            | Core::ProfileActive(_)
            | Core::InvalidProfileName(_)
            | Core::NoProfiles(_)
            | Core::SubMod(_)
            | Core::BaseModule(_)
            | Core::LoadOrder(_) => ErrorCode::InvalidInput,
            Core::Io { .. }
            | Core::Json(_)
            | Core::InvalidModInfo(_)
            | Core::InvalidRegistry(_) => ErrorCode::Io,
        };
        Self::new(code, error.to_string())
    }
}

impl From<atelier_install::Error> for AppError {
    fn from(error: atelier_install::Error) -> Self {
        use atelier_install::Error as Install;
        match error {
            Install::Core(core) => core.into(),
            Install::Unsupported(_) => Self::new(ErrorCode::NotSupported, error.to_string()),
            other => Self::new(ErrorCode::Install, other.to_string()),
        }
    }
}

impl From<atelier_modio::Error> for AppError {
    fn from(error: atelier_modio::Error) -> Self {
        use atelier_modio::Error as Modio;
        match error {
            Modio::Offline => Self::new(ErrorCode::Offline, OFFLINE_MESSAGE),
            Modio::MissingCredentials => Self::new(ErrorCode::NoApiKey, NO_API_KEY_MESSAGE),
            Modio::NotSignedIn => Self::new(ErrorCode::NotSignedIn, "Sign in to mod.io first."),
            Modio::Unauthorized => Self::new(
                ErrorCode::Unauthorized,
                "mod.io rejected your sign-in. Sign in again.",
            ),
            Modio::TermsRequired => {
                Self::new(ErrorCode::TermsRequired, "Accept mod.io's terms first.")
            }
            Modio::RateLimited { retry_after } => Self::new(
                ErrorCode::RateLimited,
                format!("mod.io is busy. Try again in {retry_after} seconds."),
            ),
            Modio::NotFound => Self::new(ErrorCode::NotFound, "That isn't on mod.io any more."),
            Modio::InvalidEmail => {
                Self::new(ErrorCode::InvalidInput, "That isn't a valid email address.")
            }
            Modio::InvalidSecurityCode => Self::new(
                ErrorCode::InvalidInput,
                "That code isn't right. Check the latest email from mod.io.",
            ),
            Modio::SecurityCodeExpired => Self::new(
                ErrorCode::InvalidInput,
                "That code has expired or was already used. Ask for a new one.",
            ),
            other => Self::new(ErrorCode::Modio, other.to_string()),
        }
    }
}
