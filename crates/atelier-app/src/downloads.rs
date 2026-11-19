//! Download jobs: fetch a mod.io file, then install it through the same preview/confirm path as a drop.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;

use atelier_core::{GameId, ModUpdate};
use atelier_modio::{DownloadJob, DownloadProgress, DownloadStatus, UpdateCheck};
use tokio::task::AbortHandle;

use crate::error::{AppError, ErrorCode, Result};
use crate::state::ModioLink;
use crate::{App, lock};

pub(crate) struct JobEntry {
    job: DownloadJob,
    game: GameId,
    abort: Option<AbortHandle>,
}

impl App {
    /// Guests download only; a signed-in account also subscribes on mod.io.
    pub async fn subscribe(&self, game: GameId, modio_id: u64) -> Result<()> {
        self.guard_writes(game)?;
        if self.has_token()? {
            self.client().subscribe(game, modio_id).await?;
        }
        self.start_job(game, modio_id);
        Ok(())
    }

    /// Installs every subscription that isn't installed here yet.
    pub async fn sync_subscriptions(&self, game: GameId) -> Result<()> {
        if !self.has_token()? {
            return Err(AppError::new(
                ErrorCode::NotSignedIn,
                "Sign in to mod.io to sync your subscriptions.",
            ));
        }
        self.guard_writes(game)?;
        let subscriptions = self.client().subscriptions(game).await?;
        self.start_missing(game, subscriptions.iter().map(|item| item.id))?;
        Ok(())
    }

    pub fn list_downloads(&self) -> Vec<DownloadJob> {
        lock(&self.0.jobs)
            .iter()
            .map(|entry| entry.job.clone())
            .collect()
    }

    pub fn retry_download(&self, job_id: &str) -> Result<()> {
        let (game, modio_id) = {
            let mut jobs = lock(&self.0.jobs);
            let entry = jobs
                .iter_mut()
                .find(|entry| entry.job.id == job_id)
                .ok_or_else(gone)?;
            if entry.job.status != DownloadStatus::Failed {
                return Ok(());
            }
            // Checked before the job changes, so a refusal never leaves it stuck as Queued.
            self.guard_writes(entry.game)?;
            entry.job.status = DownloadStatus::Queued;
            entry.job.progress = 0.0;
            entry.job.error = None;
            (entry.game, entry.job.modio_id)
        };
        self.spawn(job_id.to_owned(), game, modio_id);
        Ok(())
    }

    /// Stops the job if it is still running and drops it from the list.
    pub fn cancel_download(&self, job_id: &str) -> Result<()> {
        let mut jobs = lock(&self.0.jobs);
        let index = jobs
            .iter()
            .position(|entry| entry.job.id == job_id)
            .ok_or_else(gone)?;
        if let Some(abort) = jobs.remove(index).abort {
            abort.abort();
        }
        // Scratch space like run_job's staging: a leftover after a failed delete is harmless.
        let _ = std::fs::remove_dir_all(self.0.data_dir.join("downloads").join(job_id));
        Ok(())
    }

    /// Asks mod.io for newer files of every mod installed from it.
    pub async fn check_updates(&self, game: GameId) -> Result<()> {
        let links = self
            .persisted()
            .modio_links
            .get(&game)
            .cloned()
            .unwrap_or_default();
        let installed: Vec<(u64, u64)> = links
            .values()
            .map(|link| (link.modio_id, link.modfile_id))
            .collect();
        if installed.is_empty() {
            lock(&self.0.updates).remove(&game);
            return Ok(());
        }
        let checks = self.client().check_updates(game, &installed).await?;
        let found: HashMap<String, ModUpdate> = checks
            .into_iter()
            .filter_map(|check| match check {
                UpdateCheck::UpdateAvailable { mod_id, modfile } => {
                    let local = local_id(&links, mod_id)?;
                    let version = modfile.version.unwrap_or(modfile.filename);
                    Some((
                        local,
                        ModUpdate {
                            version,
                            modfile_id: modfile.id,
                        },
                    ))
                }
                UpdateCheck::Removed { .. } | UpdateCheck::Hidden { .. } => None,
            })
            .collect();
        lock(&self.0.updates).insert(game, found);
        Ok(())
    }

    pub fn update_mod(&self, game: GameId, mod_id: &str) -> Result<()> {
        self.guard_writes(game)?;
        let link = self
            .persisted()
            .modio_links
            .get(&game)
            .and_then(|links| links.get(mod_id).copied())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::NotFound,
                    format!("{mod_id} wasn't installed from mod.io, so it can't be updated here."),
                )
            })?;
        self.start_job(game, link.modio_id);
        Ok(())
    }

    pub fn update_all(&self, game: GameId) -> Result<()> {
        self.guard_writes(game)?;
        let pending: Vec<String> = lock(&self.0.updates)
            .get(&game)
            .map(|updates| updates.keys().cloned().collect())
            .unwrap_or_default();
        let links = self.persisted().modio_links.get(&game).cloned();
        for mod_id in pending {
            if let Some(link) = links.as_ref().and_then(|links| links.get(&mod_id)) {
                self.start_job(game, link.modio_id);
            }
        }
        Ok(())
    }

    /// Returns the pages it queued.
    pub(crate) fn start_missing(
        &self,
        game: GameId,
        modio_ids: impl Iterator<Item = u64>,
    ) -> Result<Vec<u64>> {
        let installed: HashSet<u64> = self
            .list_mods(game)?
            .into_iter()
            .filter_map(|item| item.modio_id)
            .collect();
        let queued: HashSet<u64> = lock(&self.0.jobs)
            .iter()
            .filter(|entry| !is_finished(entry.job.status))
            .map(|entry| entry.job.modio_id)
            .collect();
        let missing: Vec<u64> = modio_ids
            .filter(|modio_id| !installed.contains(modio_id) && !queued.contains(modio_id))
            .collect();
        for &modio_id in &missing {
            self.start_job(game, modio_id);
        }
        Ok(missing)
    }

    /// A page already downloading is not queued again; two installs of one mod would race.
    fn start_job(&self, game: GameId, modio_id: u64) {
        let mut jobs = lock(&self.0.jobs);
        if jobs.iter().any(|entry| {
            entry.game == game && entry.job.modio_id == modio_id && !is_finished(entry.job.status)
        }) {
            return;
        }
        let id = format!("job-{}", self.0.next_job.fetch_add(1, Ordering::Relaxed));
        jobs.push(JobEntry {
            job: DownloadJob {
                id: id.clone(),
                modio_id,
                name: format!("mod.io #{modio_id}"),
                size_bytes: 0,
                progress: 0.0,
                status: DownloadStatus::Queued,
                error: None,
            },
            game,
            abort: None,
        });
        drop(jobs);
        self.spawn(id, game, modio_id);
    }

    fn spawn(&self, job_id: String, game: GameId, modio_id: u64) {
        let app = self.clone();
        let id = job_id.clone();
        let task = tokio::spawn(async move {
            let outcome = app.run_job(&id, game, modio_id).await;
            app.edit_job(&id, |job| match outcome {
                Ok(()) => {
                    job.status = DownloadStatus::Done;
                    job.progress = 1.0;
                }
                Err(error) => {
                    job.status = DownloadStatus::Failed;
                    job.error = Some(error.message);
                }
            });
        });
        if let Some(entry) = lock(&self.0.jobs)
            .iter_mut()
            .find(|entry| entry.job.id == job_id)
        {
            entry.abort = Some(task.abort_handle());
        }
    }

    async fn run_job(&self, job_id: &str, game: GameId, modio_id: u64) -> Result<()> {
        let client = self.client();
        let detail = client.mod_detail(game, modio_id).await?;
        let name = detail.info.name.clone();
        let modfile = detail.modfile.ok_or_else(|| {
            AppError::new(
                ErrorCode::Modio,
                format!("{name} has no file to download yet."),
            )
        })?;
        self.edit_job(job_id, |job| {
            job.name.clone_from(&name);
            job.size_bytes = modfile.filesize;
            job.status = DownloadStatus::Downloading;
        });
        if detail.has_dependencies && self.settings().install_dependencies {
            let dependencies = client.dependencies(game, modio_id).await?;
            let started = self.start_missing(game, dependencies.iter().map(|item| item.id))?;
            self.hold_dependencies(game, modio_id, &started);
        }
        let staging = self.0.data_dir.join("downloads").join(job_id);
        let archive = client
            .download(&modfile, &staging, |progress| {
                self.edit_job(job_id, |job| apply_progress(job, progress));
            })
            .await?;
        self.edit_job(job_id, |job| job.status = DownloadStatus::Verifying);
        let installed =
            self.install_download(game, modio_id, modfile.id, &archive.to_string_lossy());
        // Staging is scratch space; a leftover copy is harmless, so cleanup errors are ignored.
        let _ = std::fs::remove_dir_all(&staging);
        let top_level = installed?;
        let link = ModioLink {
            modio_id,
            modfile_id: modfile.id,
        };
        self.update_persisted(|state| {
            let links = state.modio_links.entry(game).or_default();
            for mod_id in &top_level {
                // Another page's link is never taken over, even when its folder is gone.
                if links
                    .get(mod_id)
                    .is_none_or(|existing| existing.modio_id == modio_id)
                {
                    links.insert(mod_id.clone(), link);
                }
            }
        })?;
        if let Some(updates) = lock(&self.0.updates).get_mut(&game) {
            for mod_id in &top_level {
                updates.remove(mod_id);
            }
        }
        // A retry skips the already-installed mod, so the page's links name it, not this run.
        let linked: Vec<String> = self
            .persisted()
            .modio_links
            .get(&game)
            .map(|links| {
                links
                    .iter()
                    .filter(|(_, link)| link.modio_id == modio_id)
                    .map(|(mod_id, _)| mod_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        self.apply_held_off(game, modio_id, &linked)
    }

    /// Returns the top-level mod ids that landed, so sub-mods never claim the mod.io link.
    /// Only an update of this page's own mod replaces a local one; downgrades are skipped.
    fn install_download(
        &self,
        game: GameId,
        modio_id: u64,
        modfile_id: u64,
        archive: &str,
    ) -> Result<Vec<String>> {
        let preview = self.preview_install(game, &[archive.to_owned()])?;
        let links = self
            .persisted()
            .modio_links
            .get(&game)
            .cloned()
            .unwrap_or_default();
        let foreign = preview.items.iter().find(|item| {
            item.previous_version.is_some()
                && links.get(&item.mod_id).map(|link| link.modio_id) != Some(modio_id)
        });
        if let Some(item) = foreign {
            let message = format!(
                "A mod with the ModID {} is already installed and didn't come from this mod.io \
                 page. Remove it first or install it by hand.",
                item.mod_id
            );
            self.cancel_install(&preview.id)?;
            return Err(AppError::new(ErrorCode::Install, message));
        }
        // A new file from this page installs even if its author kept the manifest version.
        for item in &preview.items {
            let newer_file = links
                .get(&item.mod_id)
                .is_some_and(|link| link.modio_id == modio_id && link.modfile_id != modfile_id);
            if newer_file {
                self.0.engine.allow_reinstall(&preview.id, &item.mod_id)?;
            }
        }
        let result = self.confirm_install(&preview.id, false)?;
        Ok(preview
            .items
            .into_iter()
            .map(|item| item.mod_id)
            .filter(|mod_id| result.installed.contains(mod_id))
            .collect())
    }

    fn edit_job(&self, job_id: &str, change: impl FnOnce(&mut DownloadJob)) {
        if let Some(entry) = lock(&self.0.jobs)
            .iter_mut()
            .find(|entry| entry.job.id == job_id)
        {
            change(&mut entry.job);
        }
    }
}

fn apply_progress(job: &mut DownloadJob, progress: DownloadProgress) {
    if progress.total > 0 {
        job.progress = progress.downloaded as f64 / progress.total as f64;
    }
    if progress.status == DownloadStatus::Downloading {
        job.status = DownloadStatus::Downloading;
    }
}

fn is_finished(status: DownloadStatus) -> bool {
    matches!(status, DownloadStatus::Done | DownloadStatus::Failed)
}

fn local_id(links: &HashMap<String, ModioLink>, modio_id: u64) -> Option<String> {
    links
        .iter()
        .find(|(_, link)| link.modio_id == modio_id)
        .map(|(mod_id, _)| mod_id.clone())
}

fn gone() -> AppError {
    AppError::new(ErrorCode::NotFound, "That download is gone.")
}
