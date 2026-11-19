# Anno 117 Mod Manager (v1.9.0.0): architectural review

> Source: reverse-engineering of `/mnt/d/Anno 117 Mod Manager` (PyInstaller bundle, marshal/co_consts analysis, no code executed). 2026-11-19.

I recovered everything without writing any files. I parsed the PyInstaller archive and loaded the main script `anno117-modmanager` (704 KB, one `AnnoModManagerApp` class, 738 code objects) with marshal in memory under the local Python 3.14. I read its `co_consts` and `co_names` and never ran or imported the bundle's code. The version comes from the PYZ module `_version` (`__VERSION__='1.9.0.0'`). Line IDs below are short for `texts_english.xml` 1999101xxx.

## 1. Feature list

- **Sidebar tabs** (148–155): News, Mod Activation, Mod Browser, Collections, Manual Install, Modloader Log, Tweaking, Settings. There are also LAUNCH GAME (159) and Ko-fi, Discord and GitHub buttons (`open_kofi`, `open_discord`, `open_docs`).
- **First start:** a language picker covers the window (`_start_sequence`), then the game folder is located, then a "MOD BROWSER INTEGRATION" opt-in (001–004).
- **News:** Anno Union posts, new mod.io mods, updates to your subscribed mods, and optionally r/anno (047), all merged by date (`_fetch_all_news`). Buttons: REFRESH NEWS, ANNO UNION, Open in Mod Browser/Collections.
- **Mod Activation:**
  - Profile dropdown with Save, Save As New and Delete (019–021, 183–184, 492).
  - Search, Activate All, Load Order toggle, refresh, open folder (250–254).
  - Columns are sortable (Mod Name/Category, `toggle_sort`). Lists are "ACTIVE MODS – LOAD ORDER" and "DEACTIVATED MODS" (437–439).
  - Stats line "Mods Active/Installed … Total Size" (445).
  - Right panel: banner, version, author, Difficulty, Dependencies (Requires/Optional/Load After/Deprecates/Incompatible, 458–463), Known Issues, setup flags (453–456), folder and size, Uninstall/Unsubscribe, Activate Missing, Tweak shortcut.
  - Update badge "(!)", suppressed with a right-click (488–490, 496).
- **Mod Browser:**
  - Search, tag filter (`/tags`), sort (Most Downloads, Alphabetical, Newest, Highest Rating, Author, Last Updated, Trending, Most Subscribers; 432–436, 485–487).
  - Subscribed-only filter, Load More, a detail modal with gallery, Endorse, Visit mod.io, Install/Reinstall.
  - Update all (499–507) and Install missing (509–519) with "Stop after this mod".
- **Collections:** browse, Follow, Unfollow (with or without removing its mods, 137–138), Refresh Collection (274). Following one creates a "<name> (Collection)" preset (`_create_collection_preset`).
- **Manual Install:** a .zip file picker or a drag-and-drop area (038–040, 428).
- **Modloader Log:** shows `mod-loader.log`, with Refresh and Copy to clipboard (041–042).
- **Tweaking:** editor for each mod's `Options` block. Control types are `enum`, `toggle`, `slider` (custom `ValueSlider`), `text` and `color`/`colour` (colour chooser). Options can be grouped (`group`/`collapsed`). Reset one option, one mod, or all (111–116, 269–271).
- **Settings:**
  - Language, Tutorial Infotips, r/anno toggle, jump to the Activation tab after an install (478).
  - New-mod behaviour: Always activate / Always deactivate / Follow current state (469–471).
  - Game dir + Browse, Mod Storage (User Documents / Game Directory, 311–312), Documents override + Clear (466–468).
  - mod.io API key, Connect/Disconnect, Open Config Folder, View Debug Log, Suppressed Update Checks + Clear All.
- **App self-update prompt** (480–483).

## 2. Game and mods folder discovery

- **Windows, `find_anno_exe`:**
  - Registry: HKLM/HKCU `SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\921` and `SOFTWARE\Ubisoft\Launcher\Installs\921` (`InstallDir`).
  - Registry: `SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 3274580` (`InstallLocation`).
  - Then it globs every drive letter (`GetLogicalDrives`) under `Program Files (x86)` and `Program Files` for `Ubisoft\Ubisoft Game Launcher\games\Anno 117 - Pax Romana` and `Steam\steamapps\common\Anno 117 - Pax Romana`.
  - The target is `Bin\Win64\Anno117.exe`.
- **Linux, Wine and Steam Deck:** `_detect_wine` checks `ntdll.wine_get_version`.
  - Steam libraries are read from native, Flatpak (`.var/app/com.valvesoftware.Steam`), Snap and `/usr/share/steam` installs, plus `libraryfolders.vdf` (regex `"path"\s+"([^"]+)"`).
  - Wine prefixes come from Proton `compatdata/*/pfx`, Lutris, Heroic (`"winePrefix"`), Bottles, Faugus, PortProton and `~/.wine`.
  - Inside a prefix it searches `drive_c` for `Anno 117 - Pax Romana` to depth 3.
- **Fallback:** if nothing is found, it asks for the folder (220, 395).
- **Mods folder:**
  - Default is `Documents/Anno 117 - Pax Romana/mods`. Under Wine that is `drive_c/users/steamuser/Documents` (`_linux_prefix_documents`).
  - Otherwise it scans every drive for `*/Documents/Anno 117 - Pax Romana` and then asks (473–475).
  - A folder counts as valid if it contains `profile.txt`, `profiles/` or `active-profile.txt` (`_is_valid_mods_folder`).
  - `get_all_mod_metadata` also scans `<game>/mods`.
- **Game launch** (`_launcher_uri`, `_spawn_game`): `steam://rungameid/3274580` if the path contains `/steamapps/`; `uplay://launch/<id>/0` for Ubisoft, with the ID matched by install folder; otherwise it starts the exe directly.
- **Epic:** no Epic path or key anywhere.

## 3. Mod format, activation and load order

- **Discovery** (`_scan_folder`):
  - Reads `modinfo.json` or `modinfo.jsonc`. JSONC comments and trailing commas are stripped (`strip_jsonc_comments`).
  - Encoding fallback chain: `utf-8-sig`, `gb18030`, `big5`, `cp1252`.
  - Recurses into sub-mods (`parent_path`).
  - **Skips folders whose names start with `-` or `.`**, so the classic dash prefix still works as a hard disable.
- **Fields read:**
  - `ModID`, normalised by `_norm_mod_id`: lowercase, `[^a-z0-9]+`→`-`, `*` kept.
  - `ModName`, `Category` and `Description`, as localised dicts keyed English/German/…/Taiwanese. `Version` (default `1.0.0`), `KnownIssues`, `CreatorName`, `CreatorContact`, `Difficulty` (default `Normal`).
  - `GameSetup`: `RequiresNewGame`, `SafeToRemove`, `Multiplayer`, `Campaign`.
  - `Options`.
  - `Dependencies`: `Require`, `Optional`, `LoadAfter`, `Deprecate`, `Incompatible`.
- **Enabling and disabling** uses the game's own text files, not folder renames.
  - Newer scheme: `mods/profile.txt` holds `ActiveProfile <name>` and points at `mods/profiles/<name>.txt`.
  - Legacy scheme: one `mods/active-profile.txt`.
  - Profile file format: one ModID per line. `# <id>` means disabled, `##` starts a comment. A bare `EnableNewMods` line means new mods are on (`# EnableNewMods` turns that off). A `DisplayName <name>` line follows it. ` # not installed` is appended to stale entries (`toggle_mod_status`, `_insert_display_name`, `_remove_from_active_profile`).
  - Tweak values go to `mods/active-options.jsonc`.
- **Load order** (`_compute_load_order`, described as "verified position-for-position against mod-loader.log"):
  - Phase Normal runs first, then phase Late (`*` in `LoadAfter`).
  - Sub-mods come before top-level mods, each sorted by "[Category] ModName" in English.
  - `LoadAfter` is resolved depth-first.
  - The order is displayed only. The game decides it, and there is no reordering.
- **Conflict and dependency checks:**
  - `Incompatible` triggers a dialog: Accept Risk / Keep Disabled / Disable Conflicting Mod (117–121).
  - A missing `Require` warns at launch (419) and on uninstall (418).
  - `Deprecate` greys out the deprecated mod (`_is_deprecated_by_active`).
- **Edge states:**
  - Zero mods: shows "No mods found." (025). `_ensure_active_profile` creates a Default profile.
  - Missing modinfo: the folder is silently skipped on scan, and install fails with 227.
  - Duplicate ModID: not detected. `mod_statuses` is keyed by ID, so duplicates collapse into one entry (inferred).

## 4. Drag-and-drop install

- **Input:** `.zip` only. Both `handle_dnd_drop` (225) and the file filter `('ZIP Files','*.zip')` enforce it. No 7z, rar or folder input, and no 7z/rar libraries are in the PYZ. Without `tkinterdnd2` the app shows 429.
- **Mod root:** `ZipFile.infolist`, ignore `__MACOSX/`, find every `modinfo.json(c)`, and take the shallowest one (sorted by `/` count). Several sibling roots go through `_install_multiple_mods`. Files are extracted to `_temp_ext` and then `shutil.move`d into place.
- **Overwrite:** "The mod folder '{0}' already exists. Overwrite?" (351), then `rmtree`. The comparison is by folder name, not ModID (inferred).
- **mod.io updates:** the old folder is moved to `mkdtemp(prefix='anno117_old_')` and restored if the install fails (`_stash_existing_mod_folder`, `_finalize_installation`). The active state is kept (`_remember_activation_for_update`).
- **Backups:** saving a preset makes `<name>.bak.txt` (493); the collection preset gets `.bak`. A broken zip raises an exception that ends in 282/390, and there is no integrity check (inferred).

## 5. Network

- **mod.io:** base `https://g-11358.modapi.io/v1`, game 11358, `Authorization: Bearer <token>`.
  - `GET /authenticate/terms` (`api_key`, `X-Modio-Platform: Windows`).
  - `POST /oauth/emailrequest` (`api_key`, `email`), then `POST /oauth/emailexchange` (`api_key`, `security_code`, `terms_agreed`), returning `access_token` and `date_expires`.
  - `GET /games/11358/mods?_sort=…&_limit=51&_offset=…&_q=…&tags=…`, `?id=`, `?_sort=-date_live&_limit=10`.
  - `GET /games/11358/tags`; `GET /games/11358/mods/{id}` and its dependencies.
  - `POST/DELETE /games/11358/mods/{id}/subscribe` (`include_dependencies=true`); `POST …/ratings` (`rating`).
  - `GET /me/subscribed?game_id=11358…` (pages through all results).
  - `GET /games/11358/collections?_sort=-date_updated&_limit=50&_offset=`, `…/collections/{id}/mods?_limit=100`, `POST/DELETE …/followers`.
  - Download from `modfile.download.binary_url`, streamed. Thumbnails use `thumb_320x180`.
  - Error handling: 401 leads to a re-login; `error_ref 11002` means the API key is invalid.
- **GitHub:** `https://api.github.com/repos/Taludas/Anno-117-Mod-Manager/releases/latest` (`tag_name`, `html_url`).
- **Anno Union:** `https://www.anno-union.com/wp-json/wp/v2/posts?_embed&per_page=10`.
- **Reddit:** `https://www.reddit.com/r/anno/new.rss?limit=10` (UA `Anno117ModManager/1.0`), cached in `reddit_cache.json`.
- **External links:** `https://mod.io/me/access`, `https://mod.io/g/anno-117-pax-romana/c/`, Ko-fi `W7W8L558T`, Discord `m4e7ZanMVp`, `github.com/taludas/anno-117-mod-manager`.
- **Not called:** anno-companion and Nexus.

## 6. Profiles, localisation and theming

- **Profiles:**
  - "Default" (all mods active) and "Vanilla" (none active) are built in and can't be deleted (224).
  - Named presets live in the game's `mods/profiles/`. Legacy presets in `%APPDATA%/presets` are migrated once (`profiles_migrated`).
  - Loading a preset writes explicit entries for every installed mod (`_apply_preset_with_full_coverage`) and warns about missing ones (410).
  - No import or export beyond the `.txt` files themselves.
- **Config location:** `%APPDATA%\Anno 117 Mod Manager\`, or `$XDG_CONFIG_HOME` on Linux. Files: `settings.json`, `subscriptions.json`, `subscription_map.json` (ModID→mod.io ID), `endorsements.json`, `collection_follows.json`, `reddit_cache.json`, `debug.log` (+`.bak`).
- **Localisation:** 12 `texts_<lang>.xml` files, `{LineId: Text}` (`_load_loca`), auto-detected from the locale; a restart is needed (157).
- **Theme:**
  - Fonts: Playfair Display SC 16/13 bold for titles and headers; Marcellus 10–14 for body and UI.
  - Colours: BG `#0b192c`, section `#162a45`, hover `#253b59`, FG `#ffffff`, dim `#aaaaaa`, gold `#f1c40f`, separator `#2a3b4c`, cyan accent `#07C1D8` (hover `#09E2FF`), green `#2ecc71`, red `#e74c3c`.
  - Anno 4K icons under `data/ui/4k/...`; the copy uses an "Imperial" voice.

## 7. Weaknesses our app should fix

1. **Anno 117 only.** Strings, game ID 11358, Steam app 3274580 and Ubisoft ID 921 are all hardcoded. Anno 1800 (mod.io game id unknown: look it up, folder layout and `mod-loader.log`) is missing entirely.
2. **Zip only.** No 7z, rar or folder drops, and only one file per drop (`event.data.strip('{}')`). There is no zip-slip or size check beyond Python's own `extractall`.
3. **Overwrite by folder name.** Duplicate ModIDs in different folders are neither detected nor shown.
4. **Load order is read-only.** No per-profile order, no reorder UI, and no cycle detection in `LoadAfter`.
5. **No Epic launcher support**, and Windows discovery is just a drive glob over `Program Files`.
6. **Secrets in plain text.** The mod.io API key and token sit in `settings.json`; use the OS keychain instead. The email-code OAuth flow also needs a user-supplied API key, where an embedded app key would avoid that.
7. **Fuzzy mod.io matching.** Local mods are linked to mod.io by name or slug (`_local_mod_for_modio`), which is fragile. We should write the mod.io ID into a sidecar file on install.
8. **No backups or import/export.** No backups of profile files other than preset saves, and no shareable profile export (JSON or mod.io collection).
9. **Too much in one place.** A 700 KB single-file Tkinter monolith with threads and `after()` callbacks, an unvirtualised UI, no automatic file watcher (manual refresh, 253), and a restart to change language.
10. **News scraping is fragile.** It depends on WordPress, Reddit RSS and BeautifulSoup.

Your VERIFY check did not pass: `ls .../scratchpad/re/` exited 2 because the directory doesn't exist. My standing rules here forbid creating any files or directories, so I did the whole extraction in memory and left nothing on disk. If you want the extracted strings kept as files, the directory has to be created and the run repeated with writes allowed.
