//! Download jobs: fetch a mod.io file, then install it through the same preview/confirm path as a drop.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::time::Instant;

use atelier_core::{GameId, Mod, ModLocation, ModUpdate, Severity};
use atelier_install::{
    InstallItemStatus, InstallPreviewItem, Limits, PreviewOptions, copy_outside_atelier,
};
use atelier_modio::{DownloadJob, DownloadProgress, DownloadStatus, Modfile, UpdateCheck};
use tokio::task::AbortHandle;

use crate::error::{AppError, ErrorCode, Result};
use crate::state::ModioLink;
use crate::{App, lock};

/// How long an in-app subscribe or unsubscribe beats mod.io's subscription list.
const CHANGE_GRACE: std::time::Duration = std::time::Duration::from_secs(10 * 60);

pub(crate) struct JobEntry {
    job: DownloadJob,
    game: GameId,
    abort: Option<AbortHandle>,
}

/// Shown on a Done download whose file mod.io was too large to virus-scan.
const UNSCANNED_WARNING: &str =
    "mod.io couldn't virus-scan this file (too large), so it was installed unscanned.";

fn join_warnings(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) => Some(format!("{first} {second}")),
        (first, second) => first.or(second),
    }
}

impl App {
    /// Guests download only; a signed-in account also subscribes on mod.io.
    pub async fn subscribe(&self, game: GameId, modio_id: u64) -> Result<()> {
        self.guard_writes(game)?;
        if self.has_token()? {
            let _apply = self.0.sync_lock.lock().await;
            self.client().subscribe(game, modio_id).await?;
            self.note_change(game, modio_id, true);
            // Subscribing again is the player asking for it back after a Library uninstall.
            self.update_persisted(|state| {
                state.subscribed.entry(game).or_default().insert(modio_id);
                if let Some(declined) = state.declined.get_mut(&game) {
                    declined.remove(&modio_id);
                }
            })?;
        }
        // Signed in, an installed copy is kept: subscribing only adds it to the sync. A guest's
        // Install downloads again, which is how a guest takes a newer file.
        let signed_in_and_installed =
            self.has_token()? && self.installed_by_modio_id(game)?.contains_key(&modio_id);
        if !signed_in_and_installed {
            self.start_job(game, modio_id);
        }
        Ok(())
    }

    /// Unsubscribes on mod.io and uninstalls the page's mods, which keeps a backup.
    pub async fn unsubscribe(&self, game: GameId, modio_id: u64) -> Result<()> {
        self.guard_writes(game)?;
        let _apply = self.0.sync_lock.lock().await;
        self.client().unsubscribe(game, modio_id).await?;
        self.note_change(game, modio_id, false);
        // Forgotten only once removed: if removal fails, the next sync sees the page gone
        // from mod.io and tries again.
        self.remove_page_mods(game, modio_id)?;
        self.update_persisted(|state| {
            if let Some(subscribed) = state.subscribed.get_mut(&game) {
                subscribed.remove(&modio_id);
            }
            if let Some(declined) = state.declined.get_mut(&game) {
                declined.remove(&modio_id);
            }
        })
    }

    /// Installs every subscription that isn't installed here yet, and uninstalls the mods
    /// of subscriptions dropped since the last sync.
    pub async fn sync_subscriptions(&self, game: GameId) -> Result<()> {
        if !self.has_token()? {
            return Err(AppError::new(
                ErrorCode::NotSignedIn,
                "Sign in to mod.io to sync your subscriptions.",
            ));
        }
        self.guard_writes(game)?;
        let account = self.0.token_store.load()?;
        let fetched: BTreeSet<u64> = self
            .client()
            .subscriptions(game)
            .await?
            .iter()
            .map(|item| item.id)
            .collect();
        // The apply step, subscribe and unsubscribe take turns, so none overwrites another.
        let _apply = self.0.sync_lock.lock().await;
        // Signed out or into another account meanwhile: this answer belongs to nobody now.
        if self.0.token_store.load()? != account {
            return Ok(());
        }
        let recent = self.recent_changes(game);
        let current: BTreeSet<u64> = fetched
            .iter()
            .copied()
            .filter(|id| recent.get(id) != Some(&false))
            .chain(recent.iter().filter(|(_, on)| **on).map(|(id, _)| *id))
            .collect();
        let before = self.subscribed_set(game);
        let declined = self
            .persisted()
            .declined
            .get(&game)
            .cloned()
            .unwrap_or_default();
        // Recorded before the jobs start, so each one knows its page is a subscription and may
        // adopt a copy installed by hand. `before` stays in, so a failed removal is retried.
        self.update_persisted(|state| {
            state
                .subscribed
                .entry(game)
                .or_default()
                .extend(current.iter().copied());
        })?;
        self.start_missing(game, current.difference(&declined).copied())?;
        for &dropped in before.difference(&current) {
            self.remove_page_mods(game, dropped)?;
        }
        // Saved last, so a removal that failed is tried again by the next sync. A page no
        // longer subscribed forgets its Library uninstall, so subscribing again brings it back.
        self.update_persisted(|state| {
            if let Some(declined) = state.declined.get_mut(&game) {
                declined.retain(|page| current.contains(page));
            }
            state.subscribed.insert(game, current);
        })
    }

    /// Remembers an in-app subscribe (`true`) or unsubscribe for the grace window.
    fn note_change(&self, game: GameId, modio_id: u64, subscribed: bool) {
        lock(&self.0.recent_changes).insert((game, modio_id), (Instant::now(), subscribed));
    }

    /// In-app changes younger than the grace window, which beat mod.io's list: it can lag
    /// behind a subscribe or unsubscribe made a moment ago.
    fn recent_changes(&self, game: GameId) -> HashMap<u64, bool> {
        let mut changes = lock(&self.0.recent_changes);
        changes.retain(|_, (at, _)| at.elapsed() < CHANGE_GRACE);
        changes
            .iter()
            .filter(|((change_game, _), _)| *change_game == game)
            .map(|((_, id), (_, subscribed))| (*id, *subscribed))
            .collect()
    }

    fn subscribed_set(&self, game: GameId) -> BTreeSet<u64> {
        self.persisted()
            .subscribed
            .get(&game)
            .cloned()
            .unwrap_or_default()
    }

    /// The installed mods whose mod.io link points at this page.
    fn linked_mod_ids(&self, game: GameId, modio_id: u64) -> Vec<String> {
        self.persisted()
            .modio_links
            .get(&game)
            .map(|links| {
                links
                    .iter()
                    .filter(|(_, link)| link.modio_id == modio_id)
                    .map(|(mod_id, _)| mod_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Stops the page's pending download and uninstalls every installed mod linked to it.
    fn remove_page_mods(&self, game: GameId, modio_id: u64) -> Result<()> {
        let pending: Vec<String> = lock(&self.0.jobs)
            .iter()
            .filter(|entry| {
                entry.game == game
                    && entry.job.modio_id == modio_id
                    && !is_finished(entry.job.status)
            })
            .map(|entry| entry.job.id.clone())
            .collect();
        for job_id in pending {
            // One already installing can't be stopped: fail, so the page stays on the list and
            // the next sync removes what it installed.
            if self.cancel_download(&job_id).is_err() {
                return Err(AppError::new(
                    ErrorCode::Install,
                    "It's still installing, so it can't be removed yet. The next sync removes it.",
                ));
            }
        }
        let linked = self.linked_mod_ids(game, modio_id);
        for mod_id in linked {
            match self.uninstall_to_backup(game, &mod_id) {
                // Already gone by hand, or only a game-folder copy is left: nothing of ours
                // to remove.
                Err(error) if matches!(error.code, ErrorCode::NotFound | ErrorCode::ReadOnly) => {}
                other => other?,
            }
        }
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
            let index = jobs
                .iter()
                .position(|entry| entry.job.id == job_id)
                .ok_or_else(gone)?;
            let (game, modio_id) = (jobs[index].game, jobs[index].job.modio_id);
            if jobs[index].job.status != DownloadStatus::Failed {
                return Ok(());
            }
            // Checked before the job changes, so a refusal never leaves it stuck as Queued.
            self.guard_writes(game)?;
            // Another job already fetches this page; a second would race it into the install.
            if page_in_flight(&jobs, game, modio_id) {
                jobs.remove(index);
                return Ok(());
            }
            let job = &mut jobs[index].job;
            job.status = DownloadStatus::Queued;
            job.progress = 0.0;
            job.error = None;
            (game, modio_id)
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
        // The install step runs to the end once started, so a cancel would only hide it.
        if jobs[index].job.status == DownloadStatus::Verifying {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "It's already installing, so it can't be cancelled now.",
            ));
        }
        let entry = jobs.remove(index);
        let page = (entry.game, entry.job.modio_id);
        let still_wanted = page_in_flight(&jobs, page.0, page.1);
        drop(jobs);
        if let Some(abort) = entry.abort {
            abort.abort();
        }
        // A page installed by hand later follows the normal rules, not a collection's.
        if !still_wanted {
            lock(&self.0.held_off).remove(&page);
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
        let missing = self.missing_pages(game, modio_ids)?;
        for &modio_id in &missing {
            self.start_job(game, modio_id);
        }
        Ok(missing)
    }

    /// The pages neither installed nor already downloading.
    fn missing_pages(
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
        Ok(modio_ids
            .filter(|modio_id| !installed.contains(modio_id) && !queued.contains(modio_id))
            .collect())
    }

    /// A page already downloading is not queued again; two installs of one mod would race.
    fn start_job(&self, game: GameId, modio_id: u64) {
        let mut jobs = lock(&self.0.jobs);
        if page_in_flight(&jobs, game, modio_id) {
            return;
        }
        // The new job supersedes the page's earlier failures, so they don't pile up.
        jobs.retain(|entry| {
            !(entry.game == game
                && entry.job.modio_id == modio_id
                && entry.job.status == DownloadStatus::Failed)
        });
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
        match lock(&self.0.jobs)
            .iter_mut()
            .find(|entry| entry.job.id == job_id)
        {
            Some(entry) => entry.abort = Some(task.abort_handle()),
            // Cancelled before its handle was stored: stop it rather than let it install.
            None => task.abort(),
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
            let missing = self.missing_pages(game, dependencies.iter().map(|item| item.id))?;
            // Held before they start, so a dependency that lands first still gets its profile rules.
            self.hold_dependencies(game, modio_id, &missing);
            for &dependency in &missing {
                self.start_job(game, dependency);
            }
        }
        let staging = self.0.data_dir.join("downloads").join(job_id);
        let archive = client
            .download(&modfile, &staging, |progress| {
                self.edit_job(job_id, |job| apply_progress(job, progress));
            })
            .await?;
        // Set under the jobs lock: a job cancelled before this point installs nothing.
        if !self.edit_job(job_id, |job| job.status = DownloadStatus::Verifying) {
            let _ = std::fs::remove_dir_all(&staging);
            return Ok(());
        }
        let installed = self.install_download(game, modio_id, &modfile, &archive.to_string_lossy());
        // Staging is scratch space; a leftover copy is harmless, so cleanup errors are ignored.
        let _ = std::fs::remove_dir_all(&staging);
        let (top_level, warning) = installed?;
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
        let linked = self.linked_mod_ids(game, modio_id);
        // Like the new-mods rule: the files landed, so a failed profile write only warns. The
        // hold goes too, so a later update of this page never applies stale rules.
        let held = self
            .apply_held_off(game, modio_id, &linked)
            .err()
            .map(|error| {
                lock(&self.0.held_off).remove(&(game, modio_id));
                format!(
                    "Installed, but the collection's on/off rules weren't fully applied ({}).",
                    error.message
                )
            });
        let warning = join_warnings(warning, held);
        if warning.is_some() {
            self.edit_job(job_id, |job| job.error = warning);
        }
        // Same or older than what's installed: say so rather than report a download as done.
        if top_level.is_empty() {
            return Err(AppError::new(
                ErrorCode::Install,
                format!(
                    "{name} is already installed at this version or newer, so nothing changed."
                ),
            ));
        }
        Ok(())
    }

    /// The top-level mod ids that landed, so sub-mods never claim the mod.io link, and the
    /// install's warning.
    /// Only an update of this page's own mod replaces a local one; downgrades are skipped.
    fn install_download(
        &self,
        game: GameId,
        modio_id: u64,
        modfile: &Modfile,
        archive: &str,
    ) -> Result<(Vec<String>, Option<String>)> {
        let modfile_id = modfile.id;
        // Only a Documents copy is replaced; a game-folder copy stays beside the new one.
        // Read before the preview, so a failed scan leaves no staging behind.
        let scanned = self.scan(game)?;
        let replaceable: HashSet<String> = scanned
            .iter()
            .filter(|item| item.location == ModLocation::Documents && item.parent_id.is_none())
            .map(|item| item.id.clone())
            .collect();
        // mod.io measures the unpacked size, so a large mod gets the room it needs; the
        // archive's real size bounds that, so the figure can't open the door to a zip bomb.
        let archive_bytes = std::fs::metadata(archive).map_or(0, |meta| meta.len());
        let options = PreviewOptions {
            limits: Some(Limits::for_download(
                modfile.filesize_uncompressed,
                archive_bytes,
            )),
            modio_folder: Some(format!("{modio_id}_{modfile_id}")),
            ..PreviewOptions::default()
        };
        let preview = self.preview_install_with(game, &[archive.to_owned()], options)?;
        // Nothing to install, but the page is satisfied: link it so sync stops downloading it.
        if game == GameId::Humankind
            && let Some(note) = provided_read_only(&preview.items, &scanned)
        {
            self.cancel_install(&preview.id)?;
            let ids = preview
                .items
                .iter()
                .map(|item| item.mod_id.clone())
                .collect();
            return Ok((ids, Some(note)));
        }
        // Any Humankind item the preview refuses (a copy in mod.io's folder, a bad module) stops
        // the download with that item's reason; nothing is installed.
        if game == GameId::Humankind
            && let Some(reason) = preview
                .items
                .iter()
                .filter(|item| item.status == InstallItemStatus::Invalid)
                .flat_map(|item| &item.checks)
                .find(|check| check.level == Severity::Error)
        {
            let message = reason.message.clone();
            self.cancel_install(&preview.id)?;
            return Err(AppError::new(ErrorCode::Install, message));
        }
        let links = self
            .persisted()
            .modio_links
            .get(&game)
            .cloned()
            .unwrap_or_default();
        // A copy without any mod.io link (installed by hand or another manager) is this page's
        // when the player subscribes to the page; for a guest's Install it stays foreign.
        let subscribed = self.has_token()? && self.subscribed_set(game).contains(&modio_id);
        let foreign = preview.items.iter().find(|item| {
            replaceable.contains(&item.mod_id)
                && match links.get(&item.mod_id) {
                    Some(link) => link.modio_id != modio_id,
                    None => !subscribed,
                }
        });
        // Adopted as they are: the same version or newer is already here, so only the link is new.
        let adopted: HashSet<String> = preview
            .items
            .iter()
            .filter(|item| {
                subscribed
                    && replaceable.contains(&item.mod_id)
                    && !links.contains_key(&item.mod_id)
                    && matches!(
                        item.status,
                        InstallItemStatus::Same | InstallItemStatus::Downgrade
                    )
            })
            .map(|item| item.mod_id.clone())
            .collect();
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
        let confirmed = preview
            .items
            .iter()
            .filter(|item| {
                links
                    .get(&item.mod_id)
                    .is_some_and(|link| link.modio_id == modio_id && link.modfile_id != modfile_id)
            })
            .try_for_each(|item| self.0.engine.allow_reinstall(&preview.id, &item.mod_id))
            .map_err(AppError::from)
            .and_then(|()| self.confirm_install(&preview.id, false));
        // Nobody will come back to this preview, so its staging goes now.
        let result = match confirmed {
            Ok(result) => result,
            Err(error) => {
                let _ = self.cancel_install(&preview.id);
                return Err(error);
            }
        };
        // An adopted copy is linked only if it is still the one the preview saw.
        let adopted = if adopted.is_empty() {
            adopted
        } else {
            // Every Documents copy of each ModID, since duplicates can share one.
            let still_here: HashSet<(String, String)> = self
                .scan(game)?
                .into_iter()
                .filter(|item| item.location == ModLocation::Documents && item.parent_id.is_none())
                .map(|item| (item.id, item.version))
                .collect();
            preview
                .items
                .iter()
                .filter(|item| adopted.contains(&item.mod_id))
                .filter(|item| {
                    item.previous_version.as_ref().is_some_and(|version| {
                        still_here.contains(&(item.mod_id.clone(), version.clone()))
                    })
                })
                .map(|item| item.mod_id.clone())
                .collect()
        };
        let top_level = preview
            .items
            .into_iter()
            .map(|item| item.mod_id)
            .filter(|mod_id| result.installed.contains(mod_id) || adopted.contains(mod_id))
            .collect();
        // Only files actually installed from the download were unscanned; an adopted copy wasn't.
        let unscanned = (modfile.scan_skipped() && !result.installed.is_empty())
            .then(|| UNSCANNED_WARNING.to_owned());
        Ok((top_level, join_warnings(unscanned, result.warning)))
    }

    /// False when the job is gone (cancelled).
    fn edit_job(&self, job_id: &str, change: impl FnOnce(&mut DownloadJob)) -> bool {
        lock(&self.0.jobs)
            .iter_mut()
            .find(|entry| entry.job.id == job_id)
            .map(|entry| change(&mut entry.job))
            .is_some()
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

/// Two jobs for one page would race each other into the install.
fn page_in_flight(jobs: &[JobEntry], game: GameId, modio_id: u64) -> bool {
    jobs.iter().any(|entry| {
        entry.game == game && entry.job.modio_id == modio_id && !is_finished(entry.job.status)
    })
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

/// The note for a Humankind download whose every item a read-only copy (Workshop, game folder,
/// the game's own mod.io) already provides; a second copy would load twice, so none is added.
fn provided_read_only(items: &[InstallPreviewItem], scanned: &[Mod]) -> Option<String> {
    let mut places = items.iter().map(|item| {
        let copy = copy_outside_atelier(scanned, &item.mod_id)?;
        let read_only =
            item.status == InstallItemStatus::Invalid && copy.location != ModLocation::Documents;
        read_only.then(|| copy.location.place())
    });
    let first = places.next()??;
    if places.any(|place| place.is_none()) {
        return None;
    }
    Some(format!(
        "Already installed in the {first}; Atelier left that copy in place and linked this page to it."
    ))
}
