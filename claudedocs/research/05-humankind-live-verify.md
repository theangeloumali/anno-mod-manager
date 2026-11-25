# Humankind support: live verification on Angelo's PC (2026-11-22)

Phase 6 of `atelier-humankind-support-2026-11-21.md`.

## How it was driven

The Tauri shell can't build in WSL: it needs `dbus` and `pkg-config`, which require sudo, and there's no Windows Rust toolchain. So a throwaway driver drove the same `atelier-app` `App` the desktop shell wraps. It lives in the session scratchpad, uncommitted, and was pointed at the real folders:

- `Documents/Humankind`;
- the game at `D:\SteamLibrary\steamapps\common\Humankind`;
- the real Steam launch (`steam.exe -applaunch 1124300`).

The shared game lock was held for the whole run. `Registry.xml` was backed up first and restored afterwards; its SHA-256 prefix `29123fd39ccf4f2f` matched before and after.

The game itself is the judge: its diagnostics log (`Temporary Files/Diagnostics (...).html`) prints the list it actually loaded as `Set up new configuration ...`.

## Results

| #   | Check                                                                   | Result                                                                                                                                                                                        |
| --- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0   | Scan of the real install                                                | 29 mods (22 Workshop, 7 Community/mod.io). Every one has a module type read from its bundle, and none were skipped                                                                            |
| 0   | Health checks                                                           | All Ok: game found, Documents writable, no missing modules, no duplicate guids, BepInEx and ModListGuard present. `pagefile` and `force-vulkan` are Windows-only and absent in this Linux run |
| 1   | `set_load_order` writes the registry                                    | One line changed (`diff` against the backup), BOM kept, CRLF kept                                                                                                                             |
| 1   | Launch through `App::launch_game`                                       | The game logged `Set up new configuration project mercury,deepsea reloaded,long range nuclear missiles`, which is Atelier's order                                                             |
| 2   | Reorder, then relaunch                                                  | The game logged `Set up new configuration project mercury,long range nuclear missiles,deepsea reloaded`                                                                                       |
| 3   | Write an order naming a module that isn't installed                     | Refused: `not installed: zk missing module; Humankind would reset the mod list to vanilla`                                                                                                    |
| 3   | The registry already names a missing module (planted by hand)           | `launch_blockers` = `["zk missing module"]`; `launch_game` refused with the same message and the game did not start; health `missing-modules` = Error naming it                               |
| 4   | Drop-install a mod the game's mod.io already installed (Prussia folder) | Refused as a duplicate (`Already installed in Community\mod.io\3286172_4287251\...`), nothing written, no `Community/Atelier` folder created                                                  |
| 5   | Restore                                                                 | `Registry.xml` byte-identical to the backup                                                                                                                                                   |

## Not verified live, and why

- **Installing a new mod.io mod and seeing the game list it:**
  - **Blocked:** a real download needs the mod.io API key in `.env.local`, and reading that secret was refused in this session. Every mod I could install offline is already installed, so it would hit the duplicate guard in test 4.
  - **Coverage:** 22 engine tests in `crates/atelier-install/tests/humankind_roots.rs` and 7 app tests in `crates/atelier-app/tests/hk_install.rs` (wiremock). The game finding mods anywhere under `Community` is verified in its own code (`RuntimeManager.cs:135-160`).
  - **Owner:** Angelo, with one install from the Browse screen in the desktop build.
- **The Windows-only health rows** (pagefile, `-force-vulkan`) and the desktop shell itself: they compile in CI's Windows and Ubuntu jobs (`.github/workflows/ci.yml`), not in WSL.
