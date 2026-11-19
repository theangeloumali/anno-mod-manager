//! Whether the game is running; writes to the mods folder must wait until it exits.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::GameId;

pub fn is_game_running(game: GameId) -> bool {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let names = system
        .processes()
        .values()
        .map(|process| process.name().to_string_lossy().into_owned());
    any_named(names, game.executable_name())
}

fn any_named(names: impl IntoIterator<Item = String>, executable: &str) -> bool {
    names
        .into_iter()
        .any(|name| name.eq_ignore_ascii_case(executable))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_process_names_case_insensitively() {
        let names = ["explorer.exe".to_string(), "anno117.EXE".to_string()];
        assert!(any_named(names.clone(), "Anno117.exe"));
        assert!(!any_named(names, "Anno1800.exe"));
    }
}
