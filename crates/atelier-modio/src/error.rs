use serde::Deserialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("mod.io rejected the access token; sign in again")]
    Unauthorized,
    #[error("the mod.io Terms of Use must be accepted first")]
    TermsRequired,
    #[error("the API key is not allowed to perform this action")]
    LimitedKeyAbilities,
    #[error("not signed in to mod.io")]
    NotSignedIn,
    #[error("no mod.io API key or access token is configured")]
    MissingCredentials,
    #[error("mod.io rate limit hit; retry in {retry_after}s")]
    RateLimited { retry_after: u64 },
    #[error("mod.io is unreachable")]
    Offline,
    #[error("not found on mod.io")]
    NotFound,
    #[error("mod.io error {status} (ref {error_ref}): {message}")]
    Api {
        status: u16,
        error_ref: u32,
        message: String,
    },
    #[error("download checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("download incomplete: mod.io lists {expected} bytes, got {actual}")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("mod.io flagged modfile {modfile_id} as malware")]
    VirusDetected { modfile_id: u64 },
    #[error("mod.io has not finished a virus scan of modfile {modfile_id} (status {virus_status})")]
    NotScanned { modfile_id: u64, virus_status: u8 },
    #[error("modfile has no usable file name")]
    InvalidFileName,
    #[error("mod.io gave a download link that isn't https")]
    InsecureUrl,
    #[error("mod.io sent a response too large to read")]
    ResponseTooLarge,
    #[error("the download is larger than the {listed} bytes mod.io listed")]
    TooLarge { listed: u64 },
    #[error("invalid mod.io url: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("http error: {0}")]
    Http(#[source] reqwest::Error),
    #[error("unexpected mod.io response: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("token store error: {0}")]
    TokenStore(String),
    #[error("mod.io's list was longer than Atelier reads at once")]
    IncompleteList,
    #[error("that isn't a valid email address")]
    InvalidEmail,
    #[error("that code isn't right; check the latest email from mod.io")]
    InvalidSecurityCode,
    #[error("that code has expired or was already used; ask for a new one")]
    SecurityCodeExpired,
}

const TERMS_REQUIRED_REF: u32 = 11074;
const LIMITED_KEY_REF: u32 = 11017;

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    error_ref: u32,
    #[serde(default)]
    message: String,
}

impl Error {
    pub(crate) fn from_transport(error: reqwest::Error) -> Self {
        if error.is_connect() || error.is_timeout() {
            Self::Offline
        } else {
            // without_url: the api_key rides in the query string.
            Self::Http(error.without_url())
        }
    }

    pub(crate) fn from_status(status: u16, body: &[u8]) -> Self {
        let parsed = serde_json::from_slice::<ErrorEnvelope>(body)
            .ok()
            .map(|env| env.error);
        let error_ref = parsed.as_ref().map_or(0, |body| body.error_ref);
        match (status, error_ref) {
            (_, LIMITED_KEY_REF) => Self::LimitedKeyAbilities,
            (403, TERMS_REQUIRED_REF) => Self::TermsRequired,
            (401, _) => Self::Unauthorized,
            (404, _) => Self::NotFound,
            _ => Self::Api {
                status,
                error_ref,
                message: parsed.map(|body| body.message).unwrap_or_default(),
            },
        }
    }
}
