use atelier_app::{App, Settings};
use atelier_core::{
    GameId, GameInfo, HealthCheck, Issue, LoadOrder, LogEntry, Mod, ModOption, OptionValue, Profile,
};
use tauri::State;

use super::CommandResult;

#[tauri::command]
pub async fn list_games(app: State<'_, App>) -> CommandResult<Vec<GameInfo>> {
    app.list_games()
}

#[tauri::command]
pub async fn set_game_path(
    app: State<'_, App>,
    game: GameId,
    path: Option<String>,
) -> CommandResult<()> {
    app.set_game_path(game, path.as_deref())
}

#[tauri::command]
pub async fn list_mods(app: State<'_, App>, game: GameId) -> CommandResult<Vec<Mod>> {
    app.list_mods(game)
}

#[tauri::command]
pub async fn set_mod_active(
    app: State<'_, App>,
    game: GameId,
    mod_id: String,
    active: bool,
) -> CommandResult<()> {
    app.set_mod_active(game, &mod_id, active)
}

#[tauri::command]
pub async fn uninstall_mod(app: State<'_, App>, game: GameId, mod_id: String) -> CommandResult<()> {
    app.uninstall_mod(game, &mod_id)
}

#[tauri::command]
pub async fn restore_mod(app: State<'_, App>, game: GameId, mod_id: String) -> CommandResult<()> {
    app.restore_mod(game, &mod_id)
}

#[tauri::command]
pub async fn list_issues(app: State<'_, App>, game: GameId) -> CommandResult<Vec<Issue>> {
    app.list_issues(game)
}

#[tauri::command]
pub async fn load_order(app: State<'_, App>, game: GameId) -> CommandResult<LoadOrder> {
    app.load_order(game)
}

#[tauri::command]
pub async fn list_profiles(app: State<'_, App>, game: GameId) -> CommandResult<Vec<Profile>> {
    app.list_profiles(game)
}

#[tauri::command]
pub async fn switch_profile(app: State<'_, App>, game: GameId, name: String) -> CommandResult<()> {
    app.switch_profile(game, &name)
}

#[tauri::command]
pub async fn save_profile(app: State<'_, App>, game: GameId, name: String) -> CommandResult<()> {
    app.save_profile(game, &name)
}

#[tauri::command]
pub async fn delete_profile(app: State<'_, App>, game: GameId, name: String) -> CommandResult<()> {
    app.delete_profile(game, &name)
}

#[tauri::command]
pub async fn export_profile(
    app: State<'_, App>,
    game: GameId,
    name: String,
) -> CommandResult<String> {
    app.export_profile(game, &name)
}

#[tauri::command]
pub async fn list_options(
    app: State<'_, App>,
    game: GameId,
    mod_id: String,
) -> CommandResult<Vec<ModOption>> {
    app.list_options(game, &mod_id)
}

#[tauri::command]
pub async fn set_option(
    app: State<'_, App>,
    game: GameId,
    mod_id: String,
    key: String,
    value: Option<OptionValue>,
) -> CommandResult<()> {
    app.set_option(game, &mod_id, &key, value)
}

#[tauri::command]
pub async fn health_checks(app: State<'_, App>, game: GameId) -> CommandResult<Vec<HealthCheck>> {
    app.health_checks(game).await
}

#[tauri::command]
pub async fn loader_log(app: State<'_, App>, game: GameId) -> CommandResult<Vec<LogEntry>> {
    app.loader_log(game)
}

#[tauri::command]
pub async fn launch_game(app: State<'_, App>, game: GameId) -> CommandResult<()> {
    app.launch_game(game)
}

#[tauri::command]
pub async fn get_settings(app: State<'_, App>) -> CommandResult<Settings> {
    Ok(app.settings())
}

#[tauri::command]
pub async fn save_settings(app: State<'_, App>, settings: Settings) -> CommandResult<()> {
    app.save_settings(settings)
}
