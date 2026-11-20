//! Thin `#[tauri::command]` wrappers; every rule lives in `atelier-app`.
//! All are async so they run on Tauri's tokio runtime, which download jobs spawn onto.

mod install;
mod library;
mod modio;

pub type CommandResult<T> = Result<T, atelier_app::AppError>;

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        library::list_games,
        library::set_game_path,
        library::list_mods,
        library::set_mod_active,
        library::uninstall_mod,
        library::restore_mod,
        library::list_issues,
        library::load_order,
        library::list_profiles,
        library::switch_profile,
        library::save_profile,
        library::delete_profile,
        library::export_profile,
        library::list_options,
        library::set_option,
        library::health_checks,
        library::loader_log,
        library::launch_game,
        library::get_settings,
        library::save_settings,
        install::preview_install,
        install::confirm_install,
        install::cancel_install,
        modio::browse_modio,
        modio::modio_tags,
        modio::modio_detail,
        modio::list_collections,
        modio::follow_collection,
        modio::subscribe,
        modio::install_collection,
        modio::sync_subscriptions,
        modio::list_downloads,
        modio::retry_download,
        modio::cancel_download,
        modio::check_updates,
        modio::update_mod,
        modio::update_all,
        modio::rate_mod,
        modio::report_mod,
        modio::connectivity,
        modio::account,
        modio::accept_terms,
        modio::set_api_key,
        modio::clear_api_key,
        modio::sign_in,
        modio::sign_out,
    ]
}
