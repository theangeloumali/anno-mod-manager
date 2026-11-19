# Anno 117 + Anno 1800 Mod Manager: Domain Spec

> Source: web research (cited inline), 2026-11-19.

Two findings change the brief:

- **Anno 117 profile files have changed.** The loader docs only describe `active-profile.txt`. The most current manager, Taludas's (v1.9.0, released 2026-11-18), says newer game builds use `mods/profile.txt` plus a `mods/profiles/` folder instead, and it handles both.
- **Anno 117 has no in-game mod.io browser yet.** Ubisoft plans one for 2027 ([Anno Union](https://www.anno-union.com/?p=23284)).

I could not fetch Nexus pages (HTTP 403). Every claim has a URL or is marked unverified.

## 1. anno-companion.com Mod Manager

**Who built it:** Taludas, the fan who runs anno-companion.com. The download and repo are under his GitHub account ([page](https://anno-companion.com/117/mods/mod-manager/), [repo](https://github.com/Taludas/Anno-117-Mod-Manager)). It is written in Python with Tkinter. It runs on Windows (a packaged .exe) and on Linux (from source). The repo was created 2026-04-06 and v1.9.0 is the latest release (checked with `gh api`). It supports Anno 117 only.

**Features** ([page](https://anno-companion.com/117/mods/mod-manager/), [README](https://github.com/Taludas/Anno-117-Mod-Manager)):

- Activation list that warns about conflicts, missing dependencies and deprecated mods before launch, naming the mods involved.
- Read-only Load Order view computed from the dependency rules.
- Presets (Vanilla, Default, custom) and a copyable mod-loader log viewer.
- mod.io browser with search, tag filters, sorting and endorsements. It installs dependencies automatically and checks mod.io for updates at startup.
- Collections: installs a curated set at once and creates a preset for it.
- Manual install of a .zip by drag-and-drop or file picker.
- Tweaking tab for options a mod author exposes, with collapsible groups.
- News feed (Anno Union, mod.io, r/anno) and 12 UI languages.
- Saves the mod state, then launches the game via Ubisoft Connect or Steam.

**UX flow:** The product page has one thumbnail (`thumbnail_en.jpg`, no alt text) and no interface screenshots. The README screenshots only show the Tweaking tab: an "Economy" group open, "Visuals" collapsed, and an ungrouped option below. The tabs are Activation, Load Order, Mod Browser, Collections, Tweaking, Presets, News and Log.

**Install steps it gives:**

- The page itself only says "Download now!", which fetches `Anno.117.Mod.Manager.zip`.
- The README adds: extract it and keep the `.exe` next to the `_internal` folder. On Linux, `pip install pillow requests beautifulsoup4 tkinterdnd2` and run `anno117-modmanager.py`.

**Limitations** ([README](https://github.com/Taludas/Anno-117-Mod-Manager)):

- The browser and collections need a personal mod.io API key and an email-code login.
- Translations other than English and German are DeepL output.
- Mods installed outside the app may not link to their mod.io entries.
- Mods with malformed modinfo files may not appear.
- Drag-and-drop accepts .zip only and needs the optional `tkinterdnd2` package.
- No Anno 1800 support.

## 2. Other Anno Mod Managers

| Tool                                                                                                                                 | Stack                                  | Strengths                                                                                                                     | Gaps                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| iModYourAnno ([repo](https://github.com/anno-mods/iModYourAnno))                                                                     | C#, .NET 6 (WPF inferred from credits) | Anno 1800; installs the loader and mods; tweaking used by mods such as [harbormaster](https://github.com/adjagu/harbormaster) | Labelled WIP; last release v0.5 (2023-07-24); last push 2024-06; 43 open issues; no Anno 117                                                                                                         |
| Vortex Anno 1800 extension ([source](https://github.com/insomnious/game-anno1800), [Nexus](https://www.nexusmods.com/site/mods/521)) | Vortex extension                       | Detects Steam and Ubisoft installs; backs up overwritten files                                                                | Auto-installs the archived xforce loader, which is obsolete since GU17; a user reports mods shown active but not loaded ([forum](https://forums.nexusmods.com/topic/12984699-game-doesnt-see-mods/)) |
| Anno 1800 Mod Manager (AMM) ([Nexus](https://nexusmods.com/anno1800/mods/35))                                                        | unverified (403)                       | Mod manager, launcher and news hub                                                                                            | Installs the xforce loader (search snippet only)                                                                                                                                                     |
| In-game mod.io browser, Anno 1800 ([Anno Union](http://www.anno-union.com/?p=18341))                                                 | Native                                 | Subscribe triggers download and install; launched alongside the Aug 22, 2023 DLC                                              | mod.io content only                                                                                                                                                                                  |
| Anno 117 in-game browser                                                                                                             | n/a                                    | n/a                                                                                                                           | Planned for 2027 ([Anno Union](https://www.anno-union.com/?p=23284))                                                                                                                                 |

**The gap:** no tool manages both games, and none is a modern cross-platform desktop app.

## 3. Mod File Format

The source for the field list is the official schema, [`modinfo-schema.json`](https://github.com/anno-mods/vscode-anno-modding-tools/blob/HEAD/languages/modinfo-schema.json), together with the [modinfo spec](https://jakobharder.github.io/anno-mod-loader/modinfo).

**Required fields:**

- Anno 1800: `ModID`, `Version`, `Category`, `ModName`.
- Anno 117: `Anno`, `ModID`, `Version`, `Difficulty`, `Category`, `ModName`.

**Fields:**

- `Anno`: `7` for Anno 1800, `8` for Anno 117. Anno 117 loads only mods with `8` ([changes-117](https://jakobharder.github.io/anno-mod-loader/changes-117)).
- `ModID`: for Anno 117 it must match `^[a-z][a-z0-9]+(-[a-z0-9]+)*$`, as in `modname-creator`. Anno 1800 falls back to the folder name.
- `Version`: pattern `^([0-9]+\.)?[0-9]+\.[0-9]+$`.
- `Category` and `ModName`: localised objects. Display as `[Category] ModName`.
- `Description` and `KnownIssues[]`: localised. Language keys are English, German, Chinese, French, Italian, Japanese, Korean, Polish, Russian, Spanish, Taiwanese.
- `CreatorName`, `CreatorContact`. `ModioResourceId` exists in the Anno 1800 branch only.
- Dependencies, Anno 1800 (flat keys): `ModDependencies`, `OptionalDependencies`, `LoadAfterIds`, `IncompatibleIds`, `DeprecateIds`, and `DLCDependencies[{DLC, Dependant: required|partly|atLeastOneRequired}]` with named DLC enums.
- Dependencies, Anno 117: a `Dependencies` object with `Require`, `Optional`, `LoadAfter` (`*` means load last), `Deprecate`, `Incompatible`. Entries can be mod IDs or DLC IDs.
- Anno 117 extras:
  - `Difficulty`: one of `cheat`, `easier`, `unchanged`, `harder`.
  - `GameSetup`: `RequiresNewGame`, `SafeToRemove`, `Multiplayer`, `Campaign`.
  - Also `Options`, `Scripts`, `Custom`, and `Development` (`Dependencies`, `DeployPath`, `Bundle`).
- `Options` holds `{name: {default, label, type, values, labels}}`. User values go in `active-options.jsonc` ([options](https://jakobharder.github.io/anno-mod-loader/options)).
- `Image`: a deprecated base64 string.

**Folder layout** ([file-structure](https://jakobharder.github.io/anno-mod-loader/file-structure)):

- The mod root holds `modinfo.json`, `data/`, and optionally `banner.jpg` (16:9, for offline managers), `logo.jpg` and `README.md`.
- The modding guide also allows `banner.png` ([guide](https://anno-mods.github.io/modding-guide/getting-started/structure-of-a-mod.html)).
- Asset patches go in `data/config/export/main/asset/assets.xml` for Anno 1800 and `data/base/config/export/assets.xml` for Anno 117.

**Nested mods and collections:**

- A sub-mod has its own `modinfo.json` and turns on or off with its parent.
- When several mods share the same sub-mod, only the highest version loads.
- A collection folder is not a mod: it has no ID and no activation.
- `modinfo.json` is required in sub-folders ([install](https://jakobharder.github.io/anno-mod-loader/install)).
- Taludas also accepts `modinfo.jsonc` and strips comments.

**Valid-root rule:** the shallowest folder containing `modinfo.json` with no mod above it (Taludas source, `install_zip`).

## 4. Install Locations and Activation

**Game install detection:**

|                    | Anno 1800                                                                             | Anno 117                                                      |
| ------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Steam app ID       | 916440 ([source](https://github.com/insomnious/game-anno1800))                        | 3274580 ([Steam](https://store.steampowered.com/app/3274580)) |
| Ubisoft product ID | 4553 (`uplay://launch/4553/0`, [Lutris](https://lutris.net/games/install/29443/view)) | 921                                                           |
| Executable         | `Bin/Win64/Anno1800.exe`                                                              | `Bin/Win64/Anno117.exe`                                       |

- For Anno 117, Taludas reads `HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\921` value `InstallDir`, then `HKLM\...\Uninstall\Steam App 3274580` value `InstallLocation`.
- `Installs\4553` for Anno 1800 is my inference by analogy (unverified).
- **Steam:** read `HKCU\Software\Valve\Steam\SteamPath`, then `steamapps/libraryfolders.vdf`, then `appmanifest_<id>.acf` ([steamlocate](https://docs.rs/steamlocate/latest/src/steamlocate/lib.rs.html)).
- **Epic:** detection method is unverified. Default path is `C:\Program Files\Epic Games\Anno 1800\` ([harbormaster](https://github.com/adjagu/harbormaster)). Anno 117 is sold on Epic ([PC Gamer](https://www.pcgamer.com/games/city-builder/anno-117-launch-time-release-date)).

**Anno 1800:**

- Mod folders: `Documents\Anno 1800\mods` or `<install>\mods`.
- mod.io subscriptions go to `C:\Users\Public\mod.io\4169\mods`. That location can be changed via `RootLocalStoragePath` in `%LOCALAPPDATA%\mod.io\globalsettings.json` ([FAQ](https://www.anno-union.com/?p=18337)).
- Disable a mod by any of these ([profiles](https://jakobharder.github.io/anno-mod-loader/profiles)):
  - add `mods/activation.json` with `{"disabledIds":["ModA"]}`;
  - prefix the folder or zip name with `-` (does not work in the mod.io folder);
  - use the in-game main-menu toggle.
- Mods can also be loaded straight from .zip files.
- Log: `Documents\Anno 1800\log\mod-loader.log`.

**Anno 117:**

- Mod folders: `<Documents>/Anno 117 – Pax Romana/mods/` or `<install>/mods/`. Mods are active by default ([Anno Union](https://www.anno-union.com/?p=23284)).
- The loader docs write the paths as `<user>/Anno 117/...`. Taludas globs for `Documents/Anno 117 - Pax Romana/mods`. **The exact folder name and dash character are unverified; check for both.**
- Legacy profile file `mods/active-profile.txt`:
  - one ModID per line;
  - `#` in front disables a mod;
  - `##` lines are notes;
  - `EnableNewMods` turns new mods on by default, `# EnableNewMods` turns them off ([profiles](https://jakobharder.github.io/anno-mod-loader/profiles)).
  - One docs page instead says `<user>/Anno 117/active-profile.txt`, without `mods/`.
- Newer builds: `mods/profile.txt` names the active file inside `mods/profiles/`. Profiles may carry a display-name line and `# not installed` markers. The game writes IDs lowercased, with other characters collapsed to `-` (Taludas README and source; not in official docs).
- The `-` folder prefix also works in Anno 117.

**Load order** ([loading-order](https://jakobharder.github.io/anno-mod-loader/loading-order/)), same rules for both games:

- Two phases, normal then late. `*` puts a mod in the late phase.
- `LoadAfter` means the listed mods load first.
- Ties fall back to alphabetical order. Do not rely on it.
- Circular rules give undefined order, and missing targets produce no warning.

## 5. Loader Requirements and Game Versions

**Anno 1800:**

- Game Update 17 (April 4, 2023) built the loader into the game, so no DLL is needed ([GU17 guide](https://anno-mods.github.io/modding-guide/guides/gu17-update-guide.html), [harbormaster](https://github.com/adjagu/harbormaster)).
- The old xforce loader replaced `python35.dll` in `Bin\Win64`. Its repo was archived Aug 4, 2023, with the note "no longer required" ([xforce](https://github.com/xforce/anno1800-mod-loader)).
- Our app should detect a leftover `python35.dll` from the old loader and tell the user to verify game files. That advice comes from forum posts (unverified).

**Anno 117:**

- The loader is native from launch.
- A mod needs `Anno: 8` to load. A missing `Difficulty` is logged but the mod still loads ([changes-117](https://jakobharder.github.io/anno-mod-loader/changes-117)).
- Multiplayer requires identical mods and versions ([Anno Union](https://www.anno-union.com/?p=23284)).

**Shared code:** the loader source is [jakobharder/anno-mod-loader](https://github.com/jakobharder/anno-mod-loader), used for both 1800 and 117.

**Game-version gating:** neither game has a min/max game-version field. Use `Anno` as the gate, and treat `Version` MAJOR as the save-compatibility signal.

## 6. Recommended Detect and Install Algorithm

1. **Accept input:** .zip, .7z and .rar (via libarchive), folders, and a dropped `modinfo.json`. Unpack nested archives up to two levels deep. Skip `__MACOSX/`. Reject zip-slip paths and absolute paths.
2. **Find roots:** every `modinfo.json` or `.jsonc` with no modinfo above it becomes a separate install (same rule as Taludas). That covers a mod inside a wrapper folder and several mods in one archive.
3. **Collections:** if the archive has no modinfo but several mod roots, install each one and offer to save them as a profile.
4. **No modinfo found:** for Anno 1800 only, infer the root from the first folder containing `data/`. Warn, because mod.io-style loading requires a modinfo. For Anno 117, reject.
5. **Classify the game:**
   - `Anno` 7 or 8 decides.
   - Without it, use the asset-path layout (`data/base/config` means 117, `data/config/export/main` means 1800).
   - A wrong-game drop gets a blocking error with a "switch game" option.
6. **Validate:** parse with comments allowed and tolerate a BOM. Check the schema branch, ID format and version pattern. Resolve `Require`, `Incompatible` and `Deprecate` against installed mods and owned DLCs. DLC ownership detection is unverified, so a missing DLC should be a warning, not a block.
7. **Duplicates:**
   - Same ModID with a higher version: an update. Show the diff, keep the old copy.
   - Same ModID with a lower version: downgrade prompt.
   - Different ID in the same folder name: rename to `<ModID>`.
   - Already installed via mod.io: link the entry, do not copy.
8. **Target folder:** default to the Documents mods folder. Never write into Program Files without asking. Probe write access first, and only offer elevation for the install-dir mods folder.
9. **Write transactionally:** extract to `mods/.staging/<uuid>`, then rename atomically into place. Move a replaced mod to app-data `backups/<ModID>/<version>/`. Snapshot the profile or `activation.json` before every write. Offer one-click rollback.
10. **Activate:** write the right file for the game and build (`profile.txt`/`profiles` or `active-profile.txt` for 117, `activation.json` for 1800), using normalised IDs. Refuse to write while `Anno117.exe` or `Anno1800.exe` is running.

## 7. Differentiators for Our App

1. **Both games in one app**, with a game switcher, and both Anno 117 profile schemes.
2. **Native Tauri 2 drag-and-drop:** multi-archive, 7z and rar, nested archives, and a preview before install ("3 mods, 1 sub-mod, needs DLC X").
3. **mod.io OAuth** instead of a pasted API key, including Anno 1800's mod.io folder. Show mod.io and local mods in one list.
4. **Explained load order:** visualise the two phases, flag cycles, and show which `LoadAfter` rule placed each mod.
5. **Save safety:** use `GameSetup.RequiresNewGame`/`SafeToRemove` and `Version` MAJOR bumps to warn before changing mods on an existing save.
6. **Health check:** parse `mod-loader.log` into per-mod errors. Detect the stale Anno 1800 `python35.dll`, write-blocked folders, and mods sitting in both locations.
7. **Shareable profiles:** export and import as a mod.io collection or a JSON manifest with versions, for multiplayer parity.
8. **Generated options UI** from `Options` metadata, including the `steps` field that Taludas's Tweaking tab supports.

**Still unverified:** Epic detection, the Anno 1800 Ubisoft registry key, the Anno 117 mod.io cache path, the exact Anno 117 Documents folder name, and how the game handles a duplicate top-level ModID. Check these on a real Windows install before building.
