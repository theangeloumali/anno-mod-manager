//! Game detection, modinfo parsing, scanning, activation, load order, issues and health for Anno 117 and Anno 1800.

pub mod activation;
pub mod error;
pub mod games;
pub mod health;
pub mod humankind;
pub mod issues;
pub mod load_order;
pub mod modinfo;
pub mod options;
pub mod process;
pub mod scan;
mod snapshots;
mod text;
pub mod types;

pub use error::{Error, Result};
pub use modinfo::{ModInfo, normalize_mod_id};
pub use types::*;
