mod commands;
mod setup;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        // One copy at a time: two would race on the mods folder and the stored sign-in.
        // Registered first, as the plugin requires; a second launch focuses this window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup::manage_app)
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running the Anno Mod Manager");
}
