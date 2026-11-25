use std::path::PathBuf;

use atelier_app::{App, BepInExStatus, SaveInspection, SaveLoadReport};
use atelier_core::GameId;
use atelier_core::humankind::history::HistoryEntry;
use atelier_core::humankind::saves::SaveList;
use tauri::State;

use super::CommandResult;

#[tauri::command]
pub async fn set_load_order(
    app: State<'_, App>,
    game: GameId,
    order: Vec<String>,
) -> CommandResult<Vec<String>> {
    app.set_load_order(game, order)
}

#[tauri::command]
pub async fn launch_blockers(app: State<'_, App>, game: GameId) -> CommandResult<Vec<String>> {
    app.launch_blockers(game)
}

#[tauri::command]
pub async fn list_saves(app: State<'_, App>, game: GameId) -> CommandResult<SaveList> {
    app.list_saves(game)
}

#[tauri::command]
pub async fn inspect_save(
    app: State<'_, App>,
    game: GameId,
    path: PathBuf,
) -> CommandResult<SaveInspection> {
    app.inspect_save(game, &path)
}

#[tauri::command]
pub async fn make_save_loadable(
    app: State<'_, App>,
    game: GameId,
    path: PathBuf,
) -> CommandResult<SaveLoadReport> {
    app.make_save_loadable(game, &path)
}

#[tauri::command]
pub async fn mod_list_history(app: State<'_, App>) -> CommandResult<Vec<HistoryEntry>> {
    app.mod_list_history()
}

#[tauri::command]
pub async fn restore_history_entry(
    app: State<'_, App>,
    time: String,
    old: Vec<String>,
) -> CommandResult<String> {
    app.restore_history_entry(&time, &old)
}

#[tauri::command]
pub async fn bepinex_status(app: State<'_, App>, game: GameId) -> CommandResult<BepInExStatus> {
    app.bepinex_status(game)
}

#[tauri::command]
pub async fn install_bepinex(app: State<'_, App>, game: GameId) -> CommandResult<BepInExStatus> {
    app.install_bepinex(game).await
}

#[tauri::command]
pub async fn uninstall_bepinex(app: State<'_, App>, game: GameId) -> CommandResult<BepInExStatus> {
    app.uninstall_bepinex(game)
}
