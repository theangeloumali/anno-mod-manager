use atelier_app::{App, AppAccount, AuthMethod};
use atelier_core::GameId;
use atelier_modio::{DownloadJob, ModDetail, ModioCollection, ModioPage, ModioQuery, ReportKind};
use tauri::State;

use super::CommandResult;

#[tauri::command]
pub async fn browse_modio(app: State<'_, App>, query: ModioQuery) -> CommandResult<ModioPage> {
    app.browse(&query).await
}

#[tauri::command]
pub async fn modio_tags(app: State<'_, App>, game: GameId) -> CommandResult<Vec<String>> {
    app.modio_tags(game).await
}

#[tauri::command]
pub async fn modio_detail(
    app: State<'_, App>,
    game: GameId,
    modio_id: u64,
) -> CommandResult<ModDetail> {
    app.modio_detail(game, modio_id).await
}

#[tauri::command]
pub async fn list_collections(
    app: State<'_, App>,
    game: GameId,
) -> CommandResult<Vec<ModioCollection>> {
    app.list_collections(game).await
}

#[tauri::command]
pub async fn follow_collection(
    app: State<'_, App>,
    game: GameId,
    collection_id: u64,
    follow: bool,
) -> CommandResult<()> {
    app.follow_collection(game, collection_id, follow).await
}

#[tauri::command]
pub async fn subscribe(app: State<'_, App>, game: GameId, modio_id: u64) -> CommandResult<()> {
    app.subscribe(game, modio_id).await
}

#[tauri::command]
pub async fn install_collection(
    app: State<'_, App>,
    game: GameId,
    collection_id: u64,
) -> CommandResult<Option<String>> {
    app.install_collection(game, collection_id).await
}

#[tauri::command]
pub async fn sync_subscriptions(app: State<'_, App>, game: GameId) -> CommandResult<()> {
    app.sync_subscriptions(game).await
}

#[tauri::command]
pub async fn list_downloads(app: State<'_, App>) -> CommandResult<Vec<DownloadJob>> {
    Ok(app.list_downloads())
}

#[tauri::command]
pub async fn retry_download(app: State<'_, App>, job_id: String) -> CommandResult<()> {
    app.retry_download(&job_id)
}

#[tauri::command]
pub async fn cancel_download(app: State<'_, App>, job_id: String) -> CommandResult<()> {
    app.cancel_download(&job_id)
}

#[tauri::command]
pub async fn check_updates(app: State<'_, App>, game: GameId) -> CommandResult<()> {
    app.check_updates(game).await
}

#[tauri::command]
pub async fn update_mod(app: State<'_, App>, game: GameId, mod_id: String) -> CommandResult<()> {
    app.update_mod(game, &mod_id)
}

#[tauri::command]
pub async fn update_all(app: State<'_, App>, game: GameId) -> CommandResult<()> {
    app.update_all(game)
}

#[tauri::command]
pub async fn rate_mod(
    app: State<'_, App>,
    game: GameId,
    modio_id: u64,
    up: bool,
) -> CommandResult<()> {
    app.rate_mod(game, modio_id, up).await
}

#[tauri::command]
pub async fn report_mod(
    app: State<'_, App>,
    game: GameId,
    modio_id: u64,
    kind: ReportKind,
    summary: String,
) -> CommandResult<()> {
    app.report_mod(game, modio_id, kind, &summary).await
}

#[tauri::command]
pub async fn connectivity(app: State<'_, App>) -> CommandResult<bool> {
    Ok(app.modio_reachable().await)
}

#[tauri::command]
pub async fn account(app: State<'_, App>) -> CommandResult<AppAccount> {
    app.account().await
}

#[tauri::command]
pub async fn accept_terms(app: State<'_, App>) -> CommandResult<()> {
    app.accept_terms()
}

#[tauri::command]
pub async fn set_api_key(app: State<'_, App>, key: String) -> CommandResult<()> {
    app.set_api_key(&key).await
}

#[tauri::command]
pub async fn clear_api_key(app: State<'_, App>) -> CommandResult<()> {
    app.clear_api_key()
}

#[tauri::command]
pub async fn sign_in(
    app: State<'_, App>,
    method: AuthMethod,
    token: Option<String>,
) -> CommandResult<()> {
    app.sign_in(method, token.as_deref()).await
}

#[tauri::command]
pub async fn sign_out(app: State<'_, App>) -> CommandResult<()> {
    app.sign_out()
}
