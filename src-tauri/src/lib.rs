mod commands;
mod setup;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup::manage_app)
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running the Anno Mod Manager");
}
