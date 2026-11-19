//! Tauri commands exposing the atelier crates. The ipc-wiring slice fills this in.

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![]
}
