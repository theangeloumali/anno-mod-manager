use atelier_app::App;
use atelier_core::GameId;
use atelier_install::{InstallPreview, InstallResult};
use tauri::State;

use super::CommandResult;

#[tauri::command]
pub async fn preview_install(
    app: State<'_, App>,
    game: GameId,
    paths: Vec<String>,
) -> CommandResult<InstallPreview> {
    app.preview_install(game, &paths)
}

#[tauri::command]
pub async fn confirm_install(
    app: State<'_, App>,
    preview_id: String,
    include_downgrades: bool,
) -> CommandResult<InstallResult> {
    app.confirm_install(&preview_id, include_downgrades)
}

#[tauri::command]
pub async fn cancel_install(app: State<'_, App>, preview_id: String) -> CommandResult<()> {
    app.cancel_install(&preview_id)
}
