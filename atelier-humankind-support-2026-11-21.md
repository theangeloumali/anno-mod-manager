# Atelier: HUMANKIND support, release 1 (2026-11-21)

Plan of record, copied in Phase 0 to `/mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno/atelier-humankind-support-2026-11-21.md` (plus an `.mmd` sidecar and an HTML twin). It sits next to the existing `anno-mod-manager-plan-2026-11-19.md`.

## Context

Angelo's words:

- "I'm also worried about the reordering of the mods. It will be better if we can have a mod manager for Humankind."
- Earlier: "when I'm trying to load the game or you incorrectly add a new mod, it will cause me to lose all of the configurations".

He chose: **plan it in Atelier now**. Humankind becomes the second game in the Anno manager at `/mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno`.

The feature list (H1–H16) comes from `/mnt/c/Users/Angelo/Desktop/Work/ZKidz/Humankind/claudedocs/humankind-mod-hub-proposals-2026-11-20.md`. This plan ships the parts that answer his two complaints:

- **Order:** a load-order editor that follows the game's own rules.
- **Lost lists:** profiles, a save keeper, a check for missing mods before launch, and a restore from ModListGuard history.

It also carries the cheap reuse wins: detection, library, mod.io, drop-install and health.

## Verified facts this plan rests on (all checked 2026-11-20/21)

**Where the game keeps the mod list**

- The active list and its order are one registry value: `Documents\Humankind\Users\<id>\Registry.xml`, key `RuntimeModule/RuntimeModuleConfiguration`.
- It holds short lowercase module names joined with `/`, with `project mercury` first. ModListGuard's repro on this PC confirmed the short names.
- Saves store long names instead (`name.guid.major.minor`).

**Load-order rules** (decompiled `Amplitude.Mercury.UI.ModdingMenu`)

- Each module has a `LoadingIndex` (`:40-62`).
- At most one **Standalone**, at index 0 (`:443-456`).
- At most one **Conversion**, next (`:458-472`).
- **Extensions** are appended in any order (`:474-476`).
- Enabling a second Standalone or Conversion replaces the first.
- `MoveTo` reorders (`:486-492`).
- Every toggle reloads the whole runtime (`:483`).

**Why lists get wiped**

- If any listed module is missing at launch, the game resets the whole list to vanilla. Reproduced 2026-11-21 (Humankind plan, Phase 5).
- So a pre-launch "missing module" check prevents the wipe at its cause.

**Mod files on disk**

- Every mod folder holds `<Name>.<guid32>.<major>.<minor>`, a 4 KB UnityFS bundle (Unity 2021.3.1f1, LZ4HC-compressed) carrying the `RuntimeModule` metadata: type, author and description.
- Next to it are a `.manifest` and the big `.assetbundle`.

**mod.io**

- The game installs mod.io mods to `Documents\Humankind\Community\mod.io\<modId>_<modfileId>\` and scans `Community` recursively (`RuntimeManager.cs:135-160`).
- Humankind's mod.io game id is **2599** (`name_id` `humankind`), read from `Documents\Humankind\mod.io\cache\game_profile.data`.

**Other mod sources**

- Workshop: `D:\SteamLibrary\steamapps\workshop\content\1124300\<id>\` (24 items).
- Official: `<install>\Public\Modification`.

**Saves**

- A `.ctr` is a zip. `SaveDescriptor.bin` holds UTF-16 text with the mod list as `name.guid.major.minor`, separated by `\`.

**The game itself**

- Steam app 1124300.
- `Humankind.exe` sits at the install root, **not** under `Bin/Win64`.

**Atelier today** (HEAD `c8f8512`, 293 Rust tests)

- There is no adapter trait. The per-game config is the `impl GameId` matches in `crates/atelier-core/src/games.rs:15-64`.
- 14 Rust comparisons and 9 TS spots silently fall into the Anno path. The full list is in "Silent Anno fallbacks" below.

## Approaches

| Approach                   | What it means                                                                                                                                                                                                                                                                         | Trade-off                                                                                                        |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **A. Enum-first (chosen)** | Add `GameId::Humankind` and new enum variants (`ActivationScheme::RegistryList`, `ModLocation::Workshop/Official`). Turn every silent `==` into an exhaustive `match`, so the compiler guards the next game too. Humankind logic lives in a new `atelier-core/src/humankind/` module. | Smallest churn, and the 293 tests stay valid. Gives the same compile-time safety a trait would.                  |
| B. `GameAdapter` trait     | The 2026-11-20 audit's suggestion: move Anno behind a trait, then implement Humankind.                                                                                                                                                                                                | Rewrites every call site and test fixture for no extra safety. Today's per-game config is already a match table. |
| C. A separate app          | Fork Atelier.                                                                                                                                                                                                                                                                         | Duplicates the mod.io sync, install transaction and backups. Rejected in the proposals doc.                      |

**Why A instead of the audit's B:** the audit was written before anyone checked for an adapter seam. None exists; the existing seam is `impl GameId` plus injected closures (`crates/atelier-app/src/lib.rs:34-35`). Extending that seam is the reuse-first path.

## Assumptions (this turn's question was spent on "plan it now")

- ASSUMPTION: **Scope is release 1 = H1–H9, H11, H13 and the H16 checks.** Chose that because those answer the order and lost-list complaints, and all of it reuses Atelier. **Out of scope:**
  - H10 log reader
  - H12 share codes
  - H14 conflict scanner
  - H15 Workshop subscribe (Workshop mods are read-only in the library)
  - the City Redraw fix (waits on the overnight run)
- ASSUMPTION: **Steam only** for detection. Chose that because Angelo's copy is Steam. Epic and Game Pass paths are unverified. `ubisoft_install_id` becomes `Option`.
- ASSUMPTION: **Atelier's own mod.io installs go to `Community\Atelier\<modId>_<modfileId>\`, not the game's `Community\mod.io\`.** Chose that because the game's mod.io plugin owns its folder and its `mod_manager.data` state. The recursive `Community` scan still loads ours. Phase 6 verifies this in the game.
- ASSUMPTION: **The registry is written only while `Humankind.exe` is closed**, after a snapshot (the existing `core/snapshots.rs:27-51`), with every other key left byte-identical.
- ASSUMPTION: **No UI mock-up first.** Chose that because new screens reuse Atelier's existing shadcn/Base UI kit and the read-only load-order screen. Angelo can ask for a mock-up at plan review.
- ASSUMPTION: **The demo video has no voice**, because the narration offer was never asked this run.
- ASSUMPTION: **Commits stay local** on `feat/humankind-support`. Push and PR happen only when Angelo says so.

## Tier and route

**T3, score 10:**

- 7+ files: 3
- 4 domains (core, install, app/Tauri, React): 3
- refactor: 2
- risk 2 (writes the game's registry and installs files)

Route: team (T3). Phase 1 runs alone. Then backend slices go to @ravi and the UI to @lena, each writer in its own worktree under `Anno/.claude/worktrees/`. One review pair closes the run: @maya + @kai, since the risk is 2.

**What can run in parallel:**

- **In order:** Phase 0 → Phase 1 → Phase 2.
- **In parallel after Phase 2 (they share no file):**
  - Phase 3 (saves and vault)
  - Phase 5 (mod.io and install)
  - Phase 7 (health and BepInEx)
- **Phase 4 (profiles and load order)** needs Phase 2's registry module.
- **Phase 8 (UI)** starts once the Phase 2–7 command signatures are frozen. The signatures are written into `src/data/source.ts` first.
- **Phase 6** (live verification in the game) needs Phases 3–5 and the shared game lock.

Launch: `pnpm dev --port 5199 --strictPort` → HTTP 200 on `/` (checked 2026-11-21 from `/mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno`, server stopped afterwards).

Deploy: none. Commit locally on `feat/humankind-support`; no push, PR or merge without Angelo's word.

```taskgraph
{
  "profile": "T3",
  "nodes": [
    {"id": "p0-setup", "kind": "slice", "owner": "primary", "phase": "0",
     "files_own": ["atelier-humankind-support-2026-11-21.md", "atelier-humankind-support-2026-11-21.mmd"],
     "produces": ["branch"],
     "verify": "test \"$(git branch --show-current)\" = feat/humankind-support"},
    {"id": "p1-game-enum", "kind": "slice", "owner": "primary", "phase": "1",
     "files_own": ["crates/atelier-core/src/types.rs", "crates/atelier-core/src/games.rs", "crates/atelier-core/src/scan.rs", "crates/atelier-core/src/modinfo.rs", "crates/atelier-core/src/activation/profiles.rs", "crates/atelier-app/src/library.rs", "crates/atelier-app/src/collections.rs", "crates/atelier-install/src/lib.rs", "src/components/app/format.ts", "src/components/app/game-switcher.tsx", "src/components/app/app-store.ts", "src/data/mock-source.ts"],
     "needs": [{"node": "p0-setup", "artifact": "branch"}],
     "produces": ["game-enum"],
     "verify": "cargo test --workspace && pnpm build"},
    {"id": "p2-hk-core", "kind": "slice", "owner": "ravi", "phase": "2",
     "files_own": ["crates/atelier-core/src/humankind/mod.rs", "crates/atelier-core/src/humankind/detect.rs", "crates/atelier-core/src/humankind/module_name.rs", "crates/atelier-core/src/humankind/bundle.rs", "crates/atelier-core/src/humankind/scan.rs", "crates/atelier-core/src/humankind/registry.rs", "crates/atelier-core/src/activation/mod.rs", "crates/atelier-core/tests/humankind_core.rs"],
     "needs": [{"node": "p1-game-enum", "artifact": "game-enum"}],
     "produces": ["humankind-core"],
     "verify": "cargo test -p atelier-core"},
    {"id": "p3-saves-vault", "kind": "slice", "owner": "ravi", "phase": "3",
     "files_own": ["crates/atelier-core/src/humankind/saves.rs", "crates/atelier-install/src/backup.rs", "crates/atelier-core/tests/humankind_saves.rs", "crates/atelier-install/tests/backup_versions.rs"],
     "needs": [{"node": "p2-hk-core", "artifact": "humankind-core"}],
     "produces": ["saves-vault"],
     "verify": "cargo test -p atelier-core -p atelier-install"},
    {"id": "p4-order-profiles", "kind": "slice", "owner": "ravi", "phase": "4",
     "files_own": ["crates/atelier-core/src/humankind/load_order.rs", "crates/atelier-core/src/humankind/profiles.rs", "crates/atelier-core/src/humankind/history.rs", "crates/atelier-core/tests/humankind_load_order.rs"],
     "needs": [{"node": "p2-hk-core", "artifact": "humankind-core"}],
     "produces": ["order-profiles"],
     "verify": "cargo test -p atelier-core"},
    {"id": "p5-modio-install", "kind": "slice", "owner": "ravi", "phase": "5",
     "files_own": ["crates/atelier-modio/src/client.rs", "crates/atelier-modio/src/auth.rs", "crates/atelier-install/src/roots.rs", "crates/atelier-install/src/classify.rs", "crates/atelier-install/src/stage.rs", "crates/atelier-install/tests/humankind_roots.rs"],
     "needs": [{"node": "p2-hk-core", "artifact": "humankind-core"}],
     "produces": ["modio-install"],
     "verify": "cargo test -p atelier-modio -p atelier-install"},
    {"id": "p7-health-bepinex", "kind": "slice", "owner": "ravi", "phase": "7",
     "files_own": ["crates/atelier-core/src/health/mod.rs", "crates/atelier-core/src/health/humankind.rs", "crates/atelier-app/src/bepinex.rs", "crates/atelier-core/tests/humankind_health.rs"],
     "needs": [{"node": "p2-hk-core", "artifact": "humankind-core"}],
     "produces": ["health-bepinex"],
     "verify": "cargo test -p atelier-core -p atelier-app"},
    {"id": "merge-backend", "kind": "merge", "owner": "primary", "phase": "4",
     "files_own": ["src-tauri/src/commands/library.rs", "src-tauri/src/commands/install.rs", "src-tauri/src/commands/mod.rs", "src/data/source.ts"],
     "needs": [{"node": "p3-saves-vault", "artifact": "saves-vault"}, {"node": "p4-order-profiles", "artifact": "order-profiles"}, {"node": "p5-modio-install", "artifact": "modio-install"}, {"node": "p7-health-bepinex", "artifact": "health-bepinex"}],
     "produces": ["backend-merged"],
     "verify": "cargo test --workspace"},
    {"id": "p6-live-verify", "kind": "slice", "owner": "primary", "phase": "6",
     "files_own": ["claudedocs/research/05-humankind-live-verify.md"],
     "needs": [{"node": "merge-backend", "artifact": "backend-merged"}],
     "produces": ["live-results"],
     "verify": "test -s claudedocs/research/05-humankind-live-verify.md"},
    {"id": "p8-ui", "kind": "slice", "owner": "lena", "phase": "8",
     "files_own": ["src/features/load-order/index.tsx", "src/features/library/profile-bar.tsx", "src/features/saves/index.tsx", "src/features/health/index.tsx", "src/features/onboarding/first-run-wizard.tsx", "src/features/collections/index.tsx", "src/components/app/mod-badges.tsx"],
     "needs": [{"node": "merge-backend", "artifact": "backend-merged"}],
     "produces": ["ui"],
     "verify": "pnpm build"},
    {"id": "merge-all", "kind": "merge", "owner": "primary", "phase": "9",
     "needs": [{"node": "p6-live-verify", "artifact": "live-results"}, {"node": "p8-ui", "artifact": "ui"}],
     "produces": ["merged-tree"],
     "verify": "cargo test --workspace && pnpm build"},
    {"id": "review-qa", "kind": "verify", "owner": "maya", "phase": "9",
     "verifies": ["p0-setup", "p1-game-enum", "p2-hk-core", "p3-saves-vault", "p4-order-profiles", "p5-modio-install", "p6-live-verify", "p7-health-bepinex", "p8-ui"],
     "needs": [{"node": "merge-all", "artifact": "merged-tree"}],
     "produces": ["qa-review"],
     "verify": "cargo test --workspace"},
    {"id": "review-sec", "kind": "verify", "owner": "kai", "phase": "9",
     "verifies": ["p2-hk-core", "p3-saves-vault", "p4-order-profiles", "p5-modio-install", "p7-health-bepinex"],
     "needs": [{"node": "merge-all", "artifact": "merged-tree"}],
     "produces": ["sec-review"],
     "verify": "cargo test --workspace"},
    {"id": "close", "kind": "close", "phase": "9",
     "needs": [{"node": "review-qa", "artifact": "qa-review"}, {"node": "review-sec", "artifact": "sec-review"}]}
  ]
}
```

## Phase ledger

- [x] Phase 0: Setup
  - Branch `feat/humankind-support` from `main` (clean).
  - Check `git config user.*` is Angelo's.
  - Copy this plan into the repo, with an `.mmd` sidecar and an HTML twin (`plan-to-html.sh`).
  - Write the dispatch state file.
  - Verify: `git -C /mnt/c/Users/Angelo/Desktop/Work/ZKidz/Anno branch --show-current` prints `feat/humankind-support` and the `.html` twin exists
- [ ] Phase 1: Make the game list safe to extend (no Humankind behaviour yet)
  - TDD. Add `GameId::Humankind`, `Store`-only detection fields, `exe_subpath()` (root for Humankind, `Bin/Win64` for Anno) and `ubisoft_install_id: Option`.
  - Replace each silent comparison in "Silent Anno fallbacks" with an exhaustive `match`, so a new game fails to compile instead of quietly acting like Anno 1800.
  - Generalise `GAMES` (`crates/atelier-app/src/library.rs:18`) and the backup and prune arrays (`crates/atelier-install/src/lib.rs:216`) into `GameId::ALL`.
  - TS side: `GAME_IDS` (`src/components/app/game-switcher.tsx:7`, `src/data/source.ts:249`), the `format.ts:32-36` ternaries become a `Record<GameId,…>`, and the mock data gains Humankind entries.
  - Verify: `set -o pipefail; cargo test --workspace 2>&1 | tail -15` passes (≥293 tests) and `pnpm build` passes
- [ ] Phase 2: Humankind core, `crates/atelier-core/src/humankind/` (TDD, fixtures copied from this PC)
  - `detect.rs`: Steam 1124300 via the existing `Probe` (`games.rs:67-94`); `Documents\Humankind`; picks the `Users\<id>` folder whose `Registry.xml` is newest.
  - `module_name.rs`: parses `<Name>.<guid32>.<major>.<minor>`, maps it to the registry's short name, and back. The short-name rule is checked against the live Registry.xml and the database ModListGuard dumped.
  - `bundle.rs`: reads `RuntimeModuleType`, Author and Description from the 4 KB UnityFS bundle. It decompresses LZ4 with `lz4_flex`, a new pure-Rust dependency, and walks the type tree.
    - **Spike gate:** if the type tree can't be read inside 2 h, the type falls back to "Extension, unverified", and the UI shows a badge.
  - `scan.rs`: scan roots are `Community` (recursive, including `mod.io\` and `Atelier\`), Workshop (read-only, new `ModLocation::Workshop`) and `Public\Modification` (read-only, `ModLocation::Official`). The `Mod` record's `id` is the short name; `version` is `major.minor`.
  - `registry.rs`: reads and writes `RuntimeModule/RuntimeModuleConfiguration` in Registry.xml. It changes only that value, refuses while the game runs (`core/process.rs:34-48`), and snapshots first.
  - `activation`: a new `ActivationScheme::RegistryList`, wired into `detect_scheme` (`core/activation/mod.rs:67-76`), `read`, `apply` and `set_active`.
  - Verify: `set -o pipefail; cargo test -p atelier-core 2>&1 | tail -15` passes, including a round trip on a copy of the real Registry.xml with byte-identical other keys
- [ ] Phase 3: Save inspector, version vault, save keeper (H2, H7, H8)
  - `humankind/saves.rs`: lists `Save Files\**\*.ctr` and reads `SaveDescriptor.bin` (the `zip` and `encoding_rs` crates, already dependencies). Returns the game build, the ordered mod list and the options.
  - Per mod it reports: installed exactly, a different version installed, or missing.
  - Vault: backups (`crates/atelier-install/src/backup.rs`) gain a version-keyed restore. Today `restore_mod` takes only the newest (`backup.rs:33-35`).
    - Pruning (`backup.rs:54-70`) never deletes a version that any save on disk lists.
  - Save keeper: one command writes the save's exact list and order to the registry. A missing version is restored from the vault first; anything else is reported, never guessed.
  - Verify: `set -o pipefail; cargo test -p atelier-core -p atelier-install 2>&1 | tail -15` passes, and a fixture `.ctr` copied from `Save Files\Cloud\Cubans Turn 386.ctr` parses to its 14 mods
- [ ] Phase 4: Profiles, load-order editor, history (H11, H13)
  - `humankind/load_order.rs`: an ordered list with validation. `project mercury` comes first; at most one Standalone at index 0 and one Conversion after it; then Extensions in the user's order.
    - Enabling a second Standalone or Conversion replaces the first, as in the game (`ModdingMenu.cs:443-472`).
  - **Missing-module guard:** before writing or launching, any listed module that isn't installed blocks the action with a message. That stops the game's wipe at its cause.
  - Profiles live in app storage as JSON under Atelier's data dir: name and ordered short-name list. Apply means snapshot + registry write. "Save current as profile" is included.
  - History: reads `Documents\Humankind\ModListGuard\history.jsonl` when it exists. Each "lost" entry can be restored as a profile.
  - Tauri commands in `src-tauri/src/commands/library.rs`: `set_load_order`, `list_profiles`, `save_profile`, `apply_profile`, `delete_profile`, `mod_list_history`, `restore_history_entry`.
  - Verify: `set -o pipefail; cargo test --workspace 2>&1 | tail -15` passes, with tests for every ordering rule and the missing-module guard
- [ ] Phase 5: mod.io and drop-install (H4, H5)
  - mod.io: match arm `Humankind => 2599` (`crates/atelier-modio/src/client.rs:26-31`), plus `AUTH_GAME` (`auth.rs:11`).
  - Install target `Community\Atelier\<modId>_<modfileId>\`.
  - Install crate: `roots.rs:76-91` recognises a Humankind root (a `<Name>.<guid32>.<n>.<n>` file next to its `.assetbundle`). `classify.rs` and `stage.rs` get Humankind wording and targets. BepInEx plugin zips (a `BepInEx/plugins` tree or a lone `.dll`) are a separate kind that installs to `<install>\BepInEx\plugins\`.
  - Verify: `set -o pipefail; cargo test -p atelier-modio -p atelier-install -p atelier-app 2>&1 | tail -15` passes (wiremock for mod.io, a fixture zip per root kind)
- [ ] Phase 6: Live verification on this PC (needs the shared game lock, `~/.claude/state/game-access.lock`)
  - Back up `Registry.xml` to `.backup/`.
  - From a debug build:
    1. Apply a 2-mod profile, launch through Steam, and confirm the game loaded that order. Read the game's `Temporary Files\Diagnostics` "Set up new configuration" line.
    2. Reorder the two and confirm again.
    3. Install one mod.io mod through Atelier and confirm the game lists it.
    4. Plant a missing module and confirm Atelier blocks the launch.
  - Restore the backup afterwards.
  - Verify: the before and after `RuntimeModuleConfiguration` values and the Diagnostics lines are recorded in the plan doc's results section
- [ ] Phase 7: Health and BepInEx (H6, H9, H16 checks)
  - New functions added to `check_environment` (`crates/atelier-core/src/health/mod.rs:21-29`), each a `HealthCheck`:
    - the game was found;
    - `Documents\Humankind` is writable;
    - listed modules are missing (the wipe cause);
    - the same guid is installed twice;
    - a Huge-map save with 8+ players exists while the pagefile is system-managed or under 32 GB (read through `winreg`);
    - `-force-vulkan` is in Steam's `localconfig.vdf` launch options.
  - BepInEx one-click: the download and SHA-256 check logic from `/mnt/c/Users/Angelo/Desktop/Work/ZKidz/Humankind/scripts/install-bepinex.sh`, ported to Rust through the existing install transaction (rollback included).
    - Uninstall removes only the files it listed.
    - "Plugins loaded" comes from `BepInEx/LogOutput.log`.
  - Verify: `set -o pipefail; cargo test -p atelier-core -p atelier-app 2>&1 | tail -15` passes
- [ ] Phase 8: UI (React + shadcn/Base UI; impeccable + ui-ux-pro-max loaded first)
  - The game switcher and onboarding gain Humankind (`src/features/onboarding/first-run-wizard.tsx:38-57`).
  - Library: source badges (Community, mod.io, Workshop, Official, BepInEx) in `src/components/app/mod-badges.tsx`, a module-type chip, and a read-only lock on Workshop and Official.
  - Load order (`src/features/load-order/index.tsx`): for Humankind it becomes an editor.
    - Rows are grouped Standalone → Conversion → Extensions.
    - Up/down buttons, keyboard moves and native pointer drag (no new dependency).
    - Blocked moves explain the rule.
    - Save is disabled while the game runs.
    - The Anno view stays read-only and unchanged.
  - Profiles bar (`src/features/library/profile-bar.tsx`) for `RegistryList`: apply, save current, delete, and a "History" drawer showing ModListGuard entries.
  - Saves screen (new `src/features/saves/`): a list of saves; the detail shows build, mods in order with status colours, and a "Make this save load" button.
  - Health: the new checks plus the BepInEx install card.
  - Mock data source: Humankind fixtures for every screen.
  - Verify: `pnpm build` passes and the dev server shows 0 console errors at 375px and desktop width on every new screen (DevTools check)
- [ ] Phase 9: Close
  - Run /simplify.
  - Wave 2 review: @maya + @kai on the whole diff, then the Codex pass if logged in.
  - Reuse audit.
  - Run `dead-code-check.sh`, and prettier on every changed file.
  - Add a `CHANGELOG`/README line: Atelier now manages Anno and Humankind.
  - Write the review checklist in `~/Desktop/eor-feature-checklists/`.
  - E2E MP4 (last): the mock data source in the browser walks through load order, profile apply, the save keeper and a health warning. Frame-verified, saved to `recordings/`.
  - Vault-append.
  - Gauge the result.
  - Draw the as-built ASCII diagram.
  - Commit locally and stop (no push).
  - Verify: `set -o pipefail; cargo test --workspace 2>&1 | tail -15` and `pnpm build` pass, `bash ~/.claude/hooks/dead-code-check.sh` is clean, and every checklist row is Pass or Blocked

## Silent Anno fallbacks to convert in Phase 1

Each one becomes an exhaustive `match` with a test.

**Rust** (`core` = `crates/atelier-core/src`, `inst` = `crates/atelier-install/src`, `app` = `crates/atelier-app/src`):

- `core/games.rs:32,141,171`
- `core/scan.rs:65,132,149`
- `core/modinfo.rs:46-52`
- `core/health/mod.rs:23,153,209`
- `core/activation/profiles.rs:164-166`
- `inst/classify.rs:117,200`
- `inst/roots.rs:76-91`
- `inst/lib.rs:216`
- `app/library.rs:18,333`
- `app/collections.rs:65-66`
- `crates/atelier-modio/src/auth.rs:11`

**TypeScript:**

- `src/components/app/format.ts:32-36`
- `src/components/app/game-switcher.tsx:7`
- `src/data/source.ts:249`
- `src/features/onboarding/first-run-wizard.tsx:38-39`
- `src/components/app/app-store.ts:57`
- `src/features/collections/index.tsx:22`

## Reuse (no new code where these fit)

- **Detection and launch:** `Probe` and the Steam lookups (`core/games.rs:67-122,200-206`); launch through `steam://rungameid` (`:333-340`).
- **Game-running guard and write delay:** `core/process.rs:34-48` and `app/library.rs:22,431-445`.
- **Snapshots** before every registry write: `core/snapshots.rs:27-51`.
- **Install transaction, staging limits and backups:** `crates/atelier-install` (the transaction is reusable as-is).
- **mod.io:** sign-in, browse, subscribe and sync come from `crates/atelier-modio`, needing only one match arm.
- **Issues engine:** `core/issues.rs`. It works from `Mod.dependencies`, which stays empty for Humankind in release 1.
- **Frontend:** the `DataSource` interface (`src/data/source.ts:36-106`), the ipc and mock implementations, and the existing Library, Health and Downloads screens.

## Diagram

```ascii
                       Atelier (Tauri 2)
 ┌───────────────────────────────────────────────────────────────────────┐
 │ React UI   Library · Load order (editor) · Profiles+History · Saves · │
 │            Health · Browse/Downloads (mod.io 2599)                    │
 └───────────────────────────┬───────────────────────────────────────────┘
                             │ 49 + ~12 Tauri commands
 ┌───────────────────────────▼───────────────────────────────────────────┐
 │ atelier-app   GameId::ALL · per-game wiring · process guard · launch  │
 ├───────────────┬───────────────────────┬───────────────┬───────────────┤
 │ atelier-core  │ humankind/ (new)      │ atelier-install│ atelier-modio │
 │ games (match) │ detect · module_name  │ roots: HK +   │ 2599 arm      │
 │ activation    │ bundle (UnityFS/LZ4)  │ BepInEx kind  │ AUTH_GAME     │
 │  +RegistryList│ scan · registry       │ vault restore │               │
 │ health checks │ saves · load_order    │ by version    │               │
 │ snapshots ────┼─▶ before every write  │               │               │
 └───────────────┴──────────┬────────────┴───────┬───────┴───────────────┘
                            │ writes (game closed) │ installs
          ┌─────────────────▼──────┐   ┌──────────▼──────────────────────┐
          │ Documents\Humankind\   │   │ Community\Atelier\<id>_<file>\  │
          │ Users\<id>\Registry.xml│   │ <install>\BepInEx\plugins\      │
          │  RuntimeModule/…Config │   └─────────────────────────────────┘
          │ ModListGuard\history   │◀── read-only (restore as profile)
          │ Save Files\**\*.ctr    │◀── read-only (save keeper)
          └────────────────────────┘
   Read-only roots: Workshop 1124300 · Public\Modification · Community\mod.io
```

## Risks

- **UnityFS type-tree parsing** may fail on some bundles. The spike gate and the "unverified" badge cover it; the load order still works, because Extensions are the common case.
- **The short-name rule** (long name to registry name) is inferred. Phase 2 tests it against the live registry and ModListGuard's database dump before anything writes.
- **Two writers to the registry:** the game and Atelier. Atelier writes only while the game is closed, keeps a snapshot, and changes one key.
- **Our mod.io folder next to the game's:** the same mod subscribed in both places would load twice. The duplicate-guid health check flags it, and Phase 6 checks it in the game.
- **Product pitch:** README and PRODUCT.md say "Anno only". Phase 9 updates the wording.

## Estimates

Per the estimation guide: a mature codebase, with reuse noted.

| Task                                         | With Claude | Without   |
| -------------------------------------------- | ----------- | --------- |
| Phase 0 setup                                | 0.2h        | 0.5h      |
| Phase 1 enum-first refactor + tests          | 2h          | 8h        |
| Phase 2 Humankind core (incl. UnityFS spike) | 4h          | 16h       |
| Phase 3 saves, vault, save keeper            | 2.5h        | 10h       |
| Phase 4 profiles, load order, history        | 2.5h        | 10h       |
| Phase 5 mod.io + install                     | 1.5h        | 6h        |
| Phase 6 live verification                    | 1h          | 3h        |
| Phase 7 health + BepInEx                     | 2h          | 8h        |
| Phase 8 UI                                   | 4h          | 20h       |
| Phase 9 close (review, video, docs)          | 2h          | 6h        |
| **Total**                                    | **21.7h**   | **87.5h** |

That's about 4.0x faster, saving roughly 75% of the time.
