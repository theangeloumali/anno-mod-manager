//! Public IPC types; field names match `src/data/types.ts`.

use atelier_core::GameId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModioSort {
    Trending,
    Downloads,
    Rating,
    Newest,
    Updated,
    Subscribers,
}

impl ModioSort {
    pub(crate) fn api_value(self) -> &'static str {
        match self {
            Self::Trending => "-downloads_today",
            Self::Downloads => "-downloads_total",
            Self::Rating => "-ratings_weighted_aggregate",
            Self::Newest => "-date_live",
            Self::Updated => "-date_updated",
            Self::Subscribers => "-subscribers_total",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModioQuery {
    pub game: GameId,
    pub search: String,
    pub tags: Vec<String>,
    pub sort: ModioSort,
    pub subscribed_only: bool,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModioMod {
    pub id: u64,
    pub name_id: String,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    /// mod.io's weighted aggregate, 0.0 to 1.0.
    pub rating: f64,
    pub size_bytes: u64,
    pub logo_url: Option<String>,
    pub subscribed: bool,
    /// Filled by the install layer; mod.io does not know it.
    pub installed_mod_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModioPage {
    pub items: Vec<ModioMod>,
    pub total: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Modfile {
    pub id: u64,
    pub mod_id: u64,
    pub filename: String,
    pub version: Option<String>,
    pub filesize: u64,
    pub md5: String,
    pub binary_url: String,
    pub virus_status: u8,
    /// 1 = malware found (refused), 2 = flagged but allowed.
    pub virus_positive: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDetail {
    #[serde(flatten)]
    pub info: ModioMod,
    pub description: String,
    /// None when the mod has no live modfile yet.
    pub modfile: Option<Modfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModioCollection {
    pub id: u64,
    pub name: String,
    pub author: String,
    pub summary: String,
    /// mod.io's collection object carries no mod count.
    pub mod_count: Option<u64>,
    pub followers: u64,
    pub following: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Verifying,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub modio_id: u64,
    pub name: String,
    pub size_bytes: u64,
    pub progress: f64,
    pub status: DownloadStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub status: DownloadStatus,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub signed_in: bool,
    pub username: Option<String>,
    pub expires_at: Option<String>,
    pub terms_accepted: bool,
}

impl Account {
    pub(crate) fn signed_out() -> Self {
        Self {
            signed_in: false,
            username: None,
            expires_at: None,
            terms_accepted: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TermsLink {
    pub kind: String,
    pub text: String,
    pub url: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terms {
    pub plaintext: String,
    pub html: String,
    pub agree_label: String,
    pub disagree_label: String,
    pub links: Vec<TermsLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum UpdateCheck {
    UpdateAvailable { mod_id: u64, modfile: Modfile },
    Removed { mod_id: u64 },
    Hidden { mod_id: u64 },
}

impl UpdateCheck {
    pub fn mod_id(&self) -> u64 {
        match self {
            Self::UpdateAvailable { mod_id, .. }
            | Self::Removed { mod_id }
            | Self::Hidden { mod_id } => *mod_id,
        }
    }
}

/// mod.io report types (`POST /report`, field `type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportKind {
    Generic,
    Dmca,
    NotWorking,
    RudeContent,
    IllegalContent,
    StolenContent,
    FalseInformation,
    Other,
}

impl ReportKind {
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Generic => 0,
            Self::Dmca => 1,
            Self::NotWorking => 2,
            Self::RudeContent => 3,
            Self::IllegalContent => 4,
            Self::StolenContent => 5,
            Self::FalseInformation => 6,
            Self::Other => 7,
        }
    }
}
