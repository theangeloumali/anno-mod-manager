//! Raw mod.io response shapes (snake_case) and their mapping to the public types.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::types::{ModDetail, Modfile, ModioCollection, ModioMod, Terms, TermsLink};

pub(crate) const STATUS_ACCEPTED: u8 = 1;
pub(crate) const STATUS_DELETED: u8 = 3;

#[derive(Deserialize)]
pub(crate) struct ApiPage<T> {
    pub data: Vec<T>,
    pub result_count: u64,
    pub result_offset: u64,
    pub result_total: u64,
}

#[derive(Deserialize, Default)]
pub(crate) struct ApiUser {
    #[serde(default)]
    pub username: String,
}

#[derive(Deserialize)]
pub(crate) struct ApiAccessToken {
    pub access_token: String,
    /// Unix seconds.
    #[serde(default)]
    pub date_expires: u64,
}

#[derive(Deserialize, Default)]
pub(crate) struct ApiLogo {
    pub thumb_320x180: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ApiTag {
    pub name: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct ApiModStats {
    #[serde(default)]
    pub downloads_total: u64,
    #[serde(default)]
    pub ratings_weighted_aggregate: f64,
}

#[derive(Deserialize, Default)]
pub(crate) struct ApiFilehash {
    #[serde(default)]
    pub md5: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct ApiDownload {
    #[serde(default)]
    pub binary_url: String,
}

#[derive(Deserialize)]
pub(crate) struct ApiModfile {
    pub id: u64,
    pub mod_id: u64,
    #[serde(default)]
    pub filesize: u64,
    #[serde(default)]
    pub filehash: ApiFilehash,
    #[serde(default)]
    pub filename: String,
    pub version: Option<String>,
    #[serde(default)]
    pub virus_status: u8,
    #[serde(default)]
    pub virus_positive: u8,
    #[serde(default)]
    pub download: ApiDownload,
}

#[derive(Deserialize)]
pub(crate) struct ApiMod {
    pub id: u64,
    #[serde(default = "accepted")]
    pub status: u8,
    #[serde(default = "visible")]
    pub visible: u8,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub submitted_by: ApiUser,
    #[serde(default)]
    pub logo: ApiLogo,
    pub name: String,
    #[serde(default)]
    pub name_id: String,
    #[serde(default)]
    pub summary: String,
    pub description_plaintext: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub modfile: Option<ApiModfile>,
    #[serde(default)]
    pub tags: Vec<ApiTag>,
    #[serde(default)]
    pub stats: ApiModStats,
    #[serde(default)]
    pub dependencies: bool,
    /// Unix seconds.
    #[serde(default)]
    pub date_added: u64,
    #[serde(default)]
    pub date_updated: u64,
}

/// With the platform header, mod.io lists a file-less dependency as `"modfile":{}`, which
/// would otherwise fail the whole response on the missing `id`.
fn empty_as_none<'de, D>(deserializer: D) -> Result<Option<ApiModfile>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        Some(serde_json::Value::Object(fields)) if fields.is_empty() => Ok(None),
        Some(value) => serde_json::from_value(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn accepted() -> u8 {
    STATUS_ACCEPTED
}

fn visible() -> u8 {
    1
}

#[derive(Deserialize, Default)]
pub(crate) struct ApiCollectionStats {
    #[serde(default)]
    pub followers_total: u64,
}

#[derive(Deserialize)]
pub(crate) struct ApiCollection {
    pub id: u64,
    #[serde(default)]
    pub game_id: u64,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub submitted_by: ApiUser,
    #[serde(default)]
    pub stats: ApiCollectionStats,
}

#[derive(Deserialize)]
pub(crate) struct ApiTagOption {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Deserialize)]
pub(crate) struct ApiTermsButton {
    pub text: String,
}

#[derive(Deserialize)]
pub(crate) struct ApiTermsButtons {
    pub agree: ApiTermsButton,
    pub disagree: ApiTermsButton,
}

#[derive(Deserialize)]
pub(crate) struct ApiTermsLink {
    pub text: String,
    pub url: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Deserialize)]
pub(crate) struct ApiTerms {
    pub plaintext: String,
    pub html: String,
    pub buttons: ApiTermsButtons,
    #[serde(default)]
    pub links: BTreeMap<String, ApiTermsLink>,
}

impl ApiMod {
    pub fn is_paid(&self) -> bool {
        self.price > 0.0
    }

    pub fn is_live(&self) -> bool {
        self.status == STATUS_ACCEPTED && self.visible != 0
    }

    pub fn into_mod(self, subscribed: bool) -> ModioMod {
        ModioMod {
            id: self.id,
            name_id: self.name_id,
            name: self.name,
            summary: self.summary,
            author: self.submitted_by.username,
            tags: self.tags.into_iter().map(|tag| tag.name).collect(),
            downloads: self.stats.downloads_total,
            rating: self.stats.ratings_weighted_aggregate,
            size_bytes: self.modfile.as_ref().map_or(0, |file| file.filesize),
            logo_url: self.logo.thumb_320x180,
            date_added: self.date_added,
            date_updated: self.date_updated,
            subscribed,
            installed_mod_id: None,
        }
    }

    pub fn into_detail(mut self) -> ModDetail {
        let modfile = self.modfile.take().map(Modfile::from);
        let description = self.description_plaintext.take().unwrap_or_default();
        let has_dependencies = self.dependencies;
        let mut info = self.into_mod(false);
        info.size_bytes = modfile.as_ref().map_or(0, |file| file.filesize);
        ModDetail {
            info,
            description,
            modfile,
            has_dependencies,
        }
    }
}

impl From<ApiModfile> for Modfile {
    fn from(file: ApiModfile) -> Self {
        Self {
            id: file.id,
            mod_id: file.mod_id,
            filename: file.filename,
            version: file.version,
            filesize: file.filesize,
            md5: file.filehash.md5,
            binary_url: file.download.binary_url,
            virus_status: file.virus_status,
            virus_positive: file.virus_positive,
        }
    }
}

impl ApiCollection {
    pub fn into_collection(self, following: bool) -> ModioCollection {
        ModioCollection {
            id: self.id,
            name: self.name,
            author: self.submitted_by.username,
            summary: self.summary,
            mod_count: None,
            followers: self.stats.followers_total,
            following,
        }
    }
}

impl From<ApiTerms> for Terms {
    fn from(terms: ApiTerms) -> Self {
        Self {
            plaintext: terms.plaintext,
            html: terms.html,
            agree_label: terms.buttons.agree.text,
            disagree_label: terms.buttons.disagree.text,
            links: terms
                .links
                .into_iter()
                .map(|(kind, link)| TermsLink {
                    kind,
                    text: link.text,
                    url: link.url,
                    required: link.required,
                })
                .collect(),
        }
    }
}
