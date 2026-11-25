# Atelier: Humankind mod.io download fixes (2026-11-25)

Plan of record, copied in Phase 0 to `/mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno/atelier-humankind-modio-fixes-2026-11-25.md`. The work goes on the same branch, `feat/humankind-support`, whose PR #11 is still open.

**Validated** (all three reproduced on 2026-11-25 from Angelo's screenshot of the Downloads screen, 9 failed jobs):

1. **"No Humankind module or BepInEx plugin here"** (Emblematic Garrisons, MissileBoy).
   - The real archives, downloaded from mod.io's public file links, hold `MissileBoy/MissileBoy` + `MissileBoy/missileboy.assetbundle`: older mods have a plain module name with no `.<guid>.<major>.<minor>` suffix.
   - Atelier recognises only the long form. `ModuleName::parse` (`crates/atelier-core/src/humankind/module_name.rs:17-25`) feeds both `modules_in` (`crates/atelier-install/src/roots.rs:81-92`) and the Humankind scan (`crates/atelier-core/src/humankind/scan.rs:37-40`).
   - The game itself keys modules by plain short name, so these are loadable mods.
2. **[MAP] Ultra-Realistic earth.**
   - The archive is `Earth/[MAP] ultra-realistic earth map.hmap` plus preview PNGs and a README: a map, not a module.
   - The game reads `*.hmap` recursively from `Documents\Humankind\Maps` and `Documents\Humankind\Community` (decompiled `Amplitude.Mercury.MapEditor/MapListHelper.cs:19-23,123`).
   - Atelier has no map root.
3. **Six "Already installed in the Steam Workshop folder" failures** (Long Range Nuclear Missiles, ENCReload, Culture Super Pack, Triple Alliance Patch, DeepSea Reloaded, VIP).
   - Angelo subscribes to these on both the Workshop and mod.io.
   - The duplicate guard (`crates/atelier-install/src/classify.rs` `copy_outside_atelier`) correctly refuses a second copy.
   - But `install_download` (`crates/atelier-app/src/downloads.rs:533-575`) turns that into a **Failed** job with a Retry button, even though the mod is installed and working.
   - Nothing links the subscription to the Workshop copy, so the next sync downloads it again.

## Context

Angelo installed the CI build and synced his mod.io subscriptions; 9 of them failed. None of the fixes below may weaken the wipe guard or the duplicate guard. They make Atelier understand two more kinds of content, and stop calling a subscription that's already satisfied a failure.

## Approach

```ascii
 mod.io archive / dropped folder
        │
        ▼
 roots.rs find_roots ──▶ ModuleName::detect(file, siblings)
        │                  ├─ long:  Name.<guid>.<maj>.<min>  (as today)
        │                  └─ plain: Name + name.assetbundle beside it   ◀── fix 1
        ├─ *.hmap folder ──▶ Manifest::Map ──▶ Community\Atelier\…        ◀── fix 2
        ▼
 classify: a copy already in the Workshop / game folder / game's mod.io?
        ├─ drop install  ──▶ refused, as today
        └─ mod.io download ─▶ job Done: "Already installed from the Steam Workshop",
                              linked to that copy, so sync stops re-downloading   ◀── fix 3
```

- **Fix 1: plain modules.**
  - `ModuleName` gains `ModuleName::detect(file, siblings)`. It accepts the long form, or a plain name with no extension whose lowercase `<name>.assetbundle` sits beside it.
  - `guid` and the version become `Option`s. `short_name()` = lowercase name; `version()` = `""` when unknown.
  - The scan and `modules_in` switch to `detect`. Strict `parse` stays for save lists and registry entries.
  - The compiler finds every use of `guid`/`major`/`minor`.
- **Fix 2: maps.**
  - `roots.rs` gains `Manifest::Map`: a folder holding `*.hmap` and no module.
  - Placement is `Community\Atelier\<long name or modId_fileId>\<folder>`; the game finds it because it searches `Community` recursively.
  - The Humankind scan lists each `.hmap` as a `Mod` with category `"Map"`, id = lowercase file stem, version `""`. It's always active, never in the registry and never in the load order.
  - `set_active` on a map is refused: "Maps aren't part of the mod list; Humankind lists every installed map".
  - The Library shows a "Map" chip with no on/off switch.
  - Uninstall and restore work like any Documents mod.
- **Fix 3: a subscription the Workshop already satisfies.**
  - In `install_download`, if every item is refused only because a read-only copy (Workshop, game folder, the game's own mod.io) holds that module, cancel the preview and link the mod.io page to that copy's id. The job ends **Done** with the note "Already installed from the Steam Workshop; Atelier left that copy in place."
  - Sync then sees the page as installed and stops re-downloading. Removal already ignores read-only copies (`downloads.rs:205-210`).
  - A mixed archive, or any other refusal, fails exactly as today.

## Assumptions

- ASSUMPTION: maps install under `Community\Atelier\`, not `Documents\Humankind\Maps`. I chose that because the game reads both, and Atelier's backups and uninstall already work under `Community\Atelier`.
- ASSUMPTION: a Workshop copy satisfies a mod.io subscription even when its version differs. I chose that because installing a second copy would make the game load the module twice, and the Workshop keeps its own copy up to date.

Route: inline (one owner, the primary thread, TDD)

Launch: `pnpm dev --port 5199 --strictPort --host 127.0.0.1` → HTTP 200 on `/` (checked 2026-11-22 in this session).

Deploy: commit and push to `feat/humankind-support` (PR #11, already authorized). Merging waits on Angelo's word.

## Phase ledger

- [x] Phase 0: Setup
  - Copy this plan into the repo.
  - Copy the two small mod.io archives (MissileBoy and Emblematic Garrisons; only their small module and manifest files) into test fixtures, plus a synthetic map folder.
  - Verify: `test -f /mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno/atelier-humankind-modio-fixes-2026-11-25.md`
- [x] Phase 1: Plain modules (TDD)
  - Write failing tests for `ModuleName::detect`, for the scan listing MissileBoy as `missileboy`, and for a drop-install and a mod.io install of the MissileBoy archive landing in `Community\Atelier`. Then implement.
  - Files: `crates/atelier-core/src/humankind/{module_name,scan}.rs`, `crates/atelier-install/src/{roots,classify}.rs`, plus whatever the compiler flags for `guid`/version.
  - Verify: `set -o pipefail; cargo test --manifest-path /mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno/Cargo.toml -j 4 -p atelier-core -p atelier-modio -p atelier-install -p atelier-app 2>&1 | tail -20`
- [x] Phase 2: Maps (TDD)
  - Write failing tests: the map archive installs to `Community\Atelier\…\Earth\*.hmap`; the scan lists it as category `Map`; `set_active` on it is refused; uninstall and restore move it.
  - Implement in `roots.rs`, `classify.rs`, `install.rs`, `humankind/scan.rs` and `activation/mod.rs`.
  - UI: a Map chip and no switch, in `src/features/library/humankind-badges.tsx` and `mod-table.tsx`. Mirror it in the mock, with a UI test.
  - Verify: `set -o pipefail; cargo test --manifest-path /mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno/Cargo.toml -j 4 -p atelier-core -p atelier-modio -p atelier-install -p atelier-app 2>&1 | tail -20` and `pnpm test`
- [x] Phase 3: A subscription the Workshop already satisfies (TDD)
  - Write a failing wiremock test in `crates/atelier-app/tests/hk_install.rs`: a mod.io download of a module that has a Workshop copy ends **Done** with the note, links the page to the Workshop copy, and the next sync queues nothing. A mixed archive still fails.
  - Implement in `crates/atelier-app/src/downloads.rs`.
  - Verify: `set -o pipefail; cargo test --manifest-path /mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno/Cargo.toml -j 4 -p atelier-app 2>&1 | tail -20`
- [x] Phase 4: Close
  - Run clippy (`-D warnings`), fmt, prettier, `pnpm test`, the 375px check, the dead-code scan, and the Codex review.
  - Re-record the demo, plus a frame showing the map and a plain module in the Library.
  - Update the checklist and vault-append.
  - Commit and push to PR #11, and watch CI.
  - Verify: `gh pr checks 11` shows every check passing

## Estimates

| Task                                 | With Claude | Without |
| ------------------------------------ | ----------- | ------- |
| Phase 1 plain modules                | 1h          | 4h      |
| Phase 2 maps                         | 1.5h        | 6h      |
| Phase 3 Workshop-satisfied downloads | 1h          | 4h      |
| Phase 4 close                        | 1h          | 3h      |
| **Total**                            | **4.5h**    | **17h** |

That's about 3.8x faster, saving roughly 74% of the time.
