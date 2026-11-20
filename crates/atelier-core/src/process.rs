//! Whether the game is running; writes to the mods folder must wait until it exits.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use std::io;
use std::process::Command;

use crate::GameId;
use crate::games::LaunchTarget;

/// Starts the game the way [`crate::games::launch_target`] chose.
pub fn launch(target: &LaunchTarget) -> io::Result<()> {
    let mut command = match target {
        // explorer opens a URL through its registered handler without a console window.
        LaunchTarget::Url(url) if cfg!(windows) => {
            let mut command = Command::new("explorer");
            command.arg(url);
            command
        }
        LaunchTarget::Url(url) => {
            let mut command = Command::new("xdg-open");
            command.arg(url);
            command
        }
        LaunchTarget::Exe { exe, work_dir } => {
            let mut command = Command::new(exe);
            command.current_dir(work_dir);
            command
        }
    };
    command.spawn().map(drop)
}

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
