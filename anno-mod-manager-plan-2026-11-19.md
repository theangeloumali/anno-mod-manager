# Anno Mod Manager ("Atelier"): build plan

**Date:** 2026-11-19 · **Tier:** T2 (score 6) · **Status:** research and mock-up done. The build starts on Angelo's go-ahead.

**Decisions of record (grill round, 2026-11-19):** Tauri 2 + React/TypeScript · clickable HTML mock-up · public free release · Prototype verify mode for this session.

**Evidence this plan rests on:**

- `claudedocs/research/01-reference-app-teardown.md`: the Taludas Anno 117 Mod Manager v1.9.0.0 (`/mnt/d/Anno 117 Mod Manager`), recovered from its bytecode.
- `claudedocs/research/02-modio-integration.md`: mod.io REST API, auth, rate limits and access policy. Every endpoint is cited.
- `claudedocs/research/03-anno-mod-formats-and-competitors.md`: modinfo schemas, install paths, activation files, load order and competitors.
- `mockup/index.html` + `mockup/screens/`: the UI reference that the acceptance criteria bind to.

## Phase ledger

- [x] Phase 0a: Research and mock-up (this session)
- [ ] Phase 0b: Validate unknowns and request access
- [ ] Phase 1: Foundation (scaffold, tokens, typed IPC, DB, CI)
- [ ] Phase 2: Mod engine (detection, modinfo, activation, load order, issues)
- [ ] Phase 3: Library, profiles, load order and tweaks UI
- [ ] Phase 4: Drag-and-drop installer
- [ ] Phase 5: mod.io integration
- [ ] Phase 6: Health and diagnostics
- [ ] Phase 7: Onboarding, settings, i18n and accessibility
- [ ] Phase 8: Hardening and QA
- [ ] Phase 9: Release pipeline and launch

## What we're building

A free Windows-first desktop app (Linux/Steam Deck second) that manages mods for **both** Anno 117 and Anno 1800. One game switcher re-themes the whole app. It installs anything dropped on the window (zip, 7z, rar, folders, several at once) after showing a preview the user trusts. It browses, subscribes and updates from mod.io, and explains before launch what will break in-game.

**Why it can win against the existing tool:** Taludas's manager is the only real competitor (Anno 117 only, a 700 KB single-file Tkinter app). Its gaps are documented in research 01 §7:

- no Anno 1800;
- zip only, one file per drop;
- overwrites by folder name;
- read-only load order with no "why";
- no Epic;
- secrets in plain JSON;
- fuzzy mod.io matching;
- no import/export.

Every one of these is a phase deliverable below.

### Non-goals (v1)

- **Reordering load order.** The game decides it; we predict and explain it.
- **Nexus Mods integration, mod authoring tools, macOS.**
- **Paid mod.io mods.** They're hidden or linked out to mod.io.
- **Any write to the in-game mod.io folder** (`C:\Users\Public\mod.io\4169`).

## Architecture

```ascii
┌───────────────────────────── Atelier (Tauri 2) ──────────────────────────────┐
│                                                                              │
│  React 19 + TS (webview)                      Rust core (src-tauri)          │
│  ┌──────────────────────────┐   typed IPC    ┌────────────────────────────┐  │
│  │ features/                │◄──────────────►│ commands.rs (tauri-specta) │  │
│  │  library  order  tweaks  │  (no secrets   └─────┬──────────────────────┘  │
│  │  browse  collections     │   cross this         │                         │
│  │  downloads  health       │   line)              ▼                         │
│  │  settings  onboarding    │        ┌─────────────────────────────────────┐ │
│  │  install-preview         │        │ games/   GameAdapter trait          │ │
│  │ ui/ tokens + primitives  │        │   anno117.rs  anno1800.rs detect.rs │ │
│  │ TanStack Query + Zustand │        │ mods/    modinfo scan activation    │ │
│  └──────────────────────────┘        │          load_order issues          │ │
│                                      │ install/ archive roots preview      │ │
│                                      │          transaction backup         │ │
│                                      │ modio/   client auth download       │ │
│                                      │ store/   SQLite (rusqlite+migr.)    │ │
│                                      │ watch.rs (notify)  process.rs lock  │ │
│                                      └───┬──────────────┬──────────────┬───┘ │
└──────────────────────────────────────────┼──────────────┼──────────────┼─────┘
                                           ▼              ▼              ▼
                    Documents\Anno 117 - Pax Romana\mods   OS keychain   g-11358.modapi.io
                    Documents\Anno 1800\mods               (token)       g-4169.modapi.io
                    (+ <install>\mods, read)                             mod.io CDN
```

**Key technical decisions (Marcus):**

| Decision                                                        | Why                                                                                                                        |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| All file I/O, HTTP and tokens in Rust; React only renders       | The token never reaches the webview. Tauri capabilities allow only our own commands (Kai).                                 |
| `GameAdapter` trait, one impl per game                          | 117 and 1800 differ in schema (`Anno: 8` vs `7`), activation file, paths and Steam/Ubisoft IDs. New games become one file. |
| Hand-written typed mod.io client (reqwest + serde)              | We need ~20 endpoints. A `modio-rs` spike in Phase 0b decides whether to adopt it instead (research 02 §6).                |
| SQLite holds the mod.io ↔ local mapping (folder + ModID + md5) | Fixes the reference app's fuzzy name matching (01 §7.7) without dropping extra files into mod folders.                     |
| Load order is computed, never written                           | Mirrors the loader's documented rules: normal → late, `LoadAfter` depth-first, cycles reported (03 §4).                    |
| Writes blocked while `Anno117.exe` / `Anno1800.exe` runs        | `sysinfo` process check before every write, plus a UI lock.                                                                |
| Install = stage → validate → atomic rename → backup             | Nothing touches `mods/` until the preview is confirmed. Every replace is reversible (03 §6).                               |

**Per-game facts the adapters encode** (from research, with Phase 0b verifying what's marked †):

|                        | Anno 117                                                                | Anno 1800                                           |
| ---------------------- | ----------------------------------------------------------------------- | --------------------------------------------------- |
| mod.io game id         | 11358                                                                   | 4169                                                |
| Steam app / Ubisoft id | 3274580 / 921                                                           | 916440 / 4553†                                      |
| modinfo `Anno`         | 8                                                                       | 7 (field optional)                                  |
| Mods folder            | `Documents\Anno 117 - Pax Romana\mods`† (dash character unverified)     | `Documents\Anno 1800\mods`                          |
| Activation             | `mods\profile.txt` → `profiles\<name>.txt`, legacy `active-profile.txt` | `mods\activation.json` `{disabledIds}` + `-` prefix |
| Options                | `mods\active-options.jsonc`                                             | n/a (Options metadata where present)                |
| Log                    | `mod-loader.log`                                                        | `Documents\Anno 1800\log\mod-loader.log`            |
| Epic detection         | †                                                                       | †                                                   |

## Install flow (drag-and-drop)

```ascii
 drop (files/folders, N at once)
   │
   ▼
 stage → %LOCALAPPDATA%\Atelier\staging\<uuid>\   (7z/rar/zip via sevenz-rust / unrar / zip;
   │                                                 reject absolute + ../ paths; size cap)
   ▼
 find roots: every modinfo.json(c) with no modinfo above it  ──► 0 roots? 1800: infer from data/ (warn)
   │                                                                     117: reject with reason
   ▼
 classify game: modinfo "Anno" 7|8 → else asset layout → wrong game? offer "install for other game"
   ▼
 compare by ModID (not folder): new · update · downgrade (opt-in) · same
   ▼
 check deps: Require / Incompatible / Deprecate / DLC vs installed + active set
   ▼
 PREVIEW SHEET (nothing written yet) ── cancel ──► delete staging
   │ confirm (blocked while game runs)
   ▼
 move old copy → backups\<ModID>\<ver>\ ; atomic rename staging → mods\<ModID>
   ▼
 write activation (per adapter; keep previous on/off state) ; snapshot profile first
   ▼
 rescan → toast + library row highlighted
```

## Phase dependencies (what runs in parallel)

```ascii
 0b validate ──► 1 foundation ──┬──► 2 mod engine ──┬──► 3 library UI ────────┐
                                │                   ├──► 4 installer ────────┤
                                │                   └──► 6 health ───────────┤
                                └──► 5a modio client (Rust, no UI) ──► 5b modio UI ─┤
                                                                                    ▼
                                                     7 onboarding/i18n ──► 8 hardening ──► 9 release
```

- **Parallel:** 3, 4 and 6 run together once 2 lands (separate file sets). 5a runs alongside 2 because it shares no files with it.
- **Sequential:** 5b waits on 5a and 3, which provides the UI primitives. 8 waits on everything. 9 waits on 8.

## Phases

Estimates are for a senior engineer and include tests, review and a 25–50% buffer. The project is greenfield (1.0x maturity) and mostly novel.

### Phase 0b: Validate unknowns and request access (Marcus, Ravi, Kai)

1. Register a mod.io account and API key. Develop on **test.mod.io**. Fetch `GET /games/11358`, `GET /games/4169` and `/tags` for both games, and record `api_access_options` and the tag taxonomy.
2. **Auth spike.** Does `/oauth/emailrequest` work with a _user_ API key? Research 02 says it returns `403 / 11017`, but the reference app ships this flow, so prove it one way or the other. Also try the device-login websocket, and start the **OAuth app request** with mod.io/Ubisoft (an external wait).
3. On a real Windows install, confirm:
   - the Anno 117 Documents folder name (`-` vs `–`);
   - which activation scheme the current build writes;
   - Anno 1800 `activation.json`;
   - the Ubisoft registry key for 1800;
   - Epic manifest detection (`%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests\*.item`).
4. Legal checks:
   - RAR support: the UnRAR licence allows extraction-only use (confirm).
   - Read mod.io API Access Terms (mod.io/apiterms) for key handling and attribution.
5. Build a fixture corpus (`tests/fixtures/`), 25 archives:
   - nested wrapper;
   - multi-mod;
   - sub-mods;
   - wrong game;
   - zip-slip;
   - BOM and JSONC modinfo;
   - GB18030-encoded modinfo;
   - duplicate ModID;
   - LoadAfter cycle;
   - plus real `mod-loader.log` samples.

**Verify:** `test -s claudedocs/research/04-phase0b-findings.md`. Every † in the adapter table above is resolved or recorded as a blocker.

**Estimate:** 10h with Claude | 16h without. The mod.io approval wait is not counted.

### Phase 1: Foundation (Jordan, Lena, Ravi)

1. `pnpm create tauri-app` (Tauri 2, React 19, Vite, TS strict).
   - Plugins: dialog, opener, updater, deep-link, single-instance, log.
   - Capabilities locked to our own commands only.
2. Lift `mockup/index.html` tokens into `src/ui/tokens.css` with the two `[data-game]` skins. Build the primitives used everywhere: Button, Toggle, Chip, Stamp, Field, Drawer, Dialog, Toast, Ledger table (virtualised).
3. Typed IPC with `tauri-specta`: Rust types generate `src/lib/ipc.ts`.
4. SQLite via `rusqlite` + `refinery` migrations. Tables: `games`, `installs`, `mods`, `modio_links`, `backups`, `settings`, `http_cache`. Forward and rollback tested.
5. CI on GitHub Actions: Windows + Ubuntu matrix running `cargo fmt/clippy/test` and `pnpm lint/typecheck/test/build`.

**Verify:** `pnpm tauri build --debug` passes in CI on both OSes.

**Estimate:** 14h with Claude | 36h without.

### Phase 2: Mod engine, Rust (Ravi; test slice Maya)

1. `games/detect.rs`: Ubisoft registry (`Installs\921`, `\4553`), Steam (`steamlocate`: libraryfolders + appmanifest), Epic manifests, a manual override, and Linux Proton/Lutris/Heroic prefixes (ported from research 01 §2).
2. `mods/modinfo.rs`:
   - one tolerant parser: JSONC, BOM, trailing commas, encoding fallback utf-8 → gb18030 → big5 → cp1252;
   - both schemas (flat 1800 keys and the 117 `Dependencies` object);
   - ModID normalisation exactly as the game does it.
3. `mods/scan.rs`: walks both mods folders, sub-mods and `-`/`.` prefixed folders, and flags duplicate ModIDs (a gap in the reference app).
4. `mods/activation.rs`:
   - 117 `profile.txt`/`profiles/` and the legacy `active-profile.txt` (`#`, `##`, `EnableNewMods`, `DisplayName`);
   - 1800 `activation.json` + dash prefix;
   - snapshot before every write.
5. `mods/load_order.rs` + `issues.rs`: two phases, LoadAfter DFS with cycle report, Require/Optional/Incompatible/Deprecate/DLC. Golden-tested against real `mod-loader.log` order.
6. `process.rs` (game-running lock) and `watch.rs` (`notify`: external changes refresh the library).

**Verify:** `cargo test -p atelier-core` passes, including the golden load-order tests on the fixture corpus.

**Estimate:** 28h with Claude | 72h without.

### Phase 3: Library, profiles, load order and tweaks UI (Lena; design review Priya)

1. Library ledger: toggles, search, filter chips (All/Active/Inactive/Updates/Issues), state stamps, stats line, detail panel with save-safety stamps, dependency dots and one-click fixes. Matches `mockup/screens/01-library-117.png`.
2. Profiles:
   - switch, save, save as, delete;
   - built-in Default and Vanilla can't be deleted;
   - **export/import** as `*.atelier.json` (ModID, version, mod.io id), and offer to install whatever's missing on import.
3. Load order view with a "why" per row and a cycle banner (`02-order-117.png`, `03-order-1800-cycle.png`).
4. Tweaks: a form generated from `Options` (toggle, enum, slider with steps, text, colour, groups), written to `active-options.jsonc`.
5. Game switcher re-theme, and the Launch button: `steam://rungameid/…` / `uplay://launch/…/0` / exe, with a pre-launch issue summary.

**Verify:** `pnpm test` (component tests) + `pnpm build`. Check at 960px and 375px with 0 console errors.

**Estimate:** 22h with Claude | 60h without.

### Phase 4: Drag-and-drop installer (Ravi engine, Lena UI; Kai reviews)

1. `install/archive.rs`:
   - zip (`zip`), 7z (`sevenz-rust`), rar (`unrar`, if Phase 0b clears it);
   - nested archives up to 2 levels;
   - zip-slip and absolute-path rejection, size and file-count caps.
2. `install/roots.rs` + `preview.rs`: implements the install-flow diagram above and returns an `InstallPreview` per root.
3. `install/transaction.rs` + `backup.rs`: staging, atomic rename, backups with retention (7 / 30 days / forever / off), one-click rollback.
4. UI:
   - full-window drop target (Tauri `onDragDropEvent`, multiple paths);
   - preview sheet with new/update/downgrade/wrong-game states (`04-*.png`);
   - "Install from file" picker as a fallback.

**Verify:** `cargo test -p atelier-core install::` over the 25-archive fixture corpus. Zip-slip fixtures must be rejected.

**Estimate:** 20h with Claude | 52h without.

### Phase 5: mod.io integration (5a Ravi, Rust · 5b Lena, UI · Kai reviews auth)

**5a, client (runs alongside Phase 2):**

1. `modio/client.rs`:
   - `g-{game}.modapi.io/v1`, never `api.mod.io`;
   - `retry-after` backoff, an `http_cache` table, offline detection.
2. `modio/auth.rs`:
   - Terms screen (`GET /authenticate/terms`) before any login;
   - methods in order: **(1) pasted access token** (works today, the MVP), **(2) device login** (websocket, if confirmed), **(3) Sign in with mod.io** OAuth + deep link (once approved; needs a tiny token-exchange endpoint, owned by Ravi and Jordan);
   - token stored in the OS keychain (`keyring`); a 401 clears it and prompts a re-login.
3. `modio/download.rs`: streamed download, md5 check against `filehash.md5`, resume via range (if the CDN supports it), then hand off to the Phase 4 transaction.
4. Update checks: `GET /games/{id}/mods?id-in=<≤100>` at start and hourly, plus a `GET /me/subscribed` diff when logged in. A removed or hidden mod becomes a stamp, not an error.

**5b, UI:**

1. Browse grid: search, runtime-fetched tags, sort, subscribed-only filter.
2. Mod drawer: gallery, dependencies, subscribe & install, rate, report.
3. Collections: follow, "Install all + create profile".
4. Downloads queue.
5. Signed-out and offline states, and "Powered by mod.io" attribution per the API terms (`05-*`, `06-*` screens).

**Verify:** `cargo test -p atelier-core modio::` against recorded test.mod.io fixtures, plus one live smoke against test.mod.io.

**Estimate:** 34h with Claude | 84h without. OAuth approval is an external wait.

### Phase 6: Health and diagnostics (Ravi, Lena)

1. `mod-loader.log` parser: per-mod error and warning cards, timestamps, "Fix" deep-links into the library.
2. Environment checks:
   - game found;
   - mods folder writable;
   - stale Anno 1800 `python35.dll`;
   - same ModID in our folder and the in-game mod.io folder;
   - activation-scheme detection.
3. Debug bundle export: logs + settings, redacted, with no token.

**Verify:** `cargo test -p atelier-core health::` on the fixture logs.

**Estimate:** 10h with Claude | 26h without.

### Phase 7: Onboarding, settings, i18n and accessibility (Lena, Priya, Elena)

1. First-run wizard (`08-wizard.png`): detected games, then mods folders, then mod.io (skippable), then done. Existing activation state is imported, never reset.
2. Settings: paths per store, new-mod behaviour, dependency install, backups, update checks, language, update channel, diagnostics.
3. i18n with `i18next`: English and German at launch. The structure covers the 11 languages Anno ships with.
4. Accessibility:
   - keyboard paths for every action;
   - focus rings and 4.5:1 contrast in both skins;
   - reduced motion;
   - screen-reader labels on toggles and stamps.

**Verify:** `pnpm test` + an axe run in the component tests (0 serious issues).

**Estimate:** 14h with Claude | 36h without.

### Phase 8: Hardening and QA (Maya + Kai, independent of the builders)

1. Regression matrix: Steam / Ubisoft / Epic × 117 / 1800 × fresh / existing mods × online / offline / signed-out × game running.
2. Performance: 500 installed mods scroll at 60 fps (virtualised); a cold scan under 2s.
3. Kai's security review:
   - IPC capability allowlist, CSP, archive extraction limits;
   - token storage, updater signature, deep-link input validation;
   - no write path outside the configured mods folders.

**Verify:** the full CI suite passes, plus the signed-off Maya + Kai review notes.

**Estimate:** 18h with Claude | 40h without.

### Phase 9: Release pipeline and launch (Jordan, Elena; Nia optional)

1. Windows: NSIS installer, Authenticode signing (Azure Trusted Signing or an OV cert, a budget decision for Angelo). Linux: AppImage.
2. `tauri-plugin-updater` with signed manifests on GitHub Releases, stable and beta channels.
3. Docs: install guide, FAQ, and a privacy policy that links mod.io's Terms and Privacy (required by research 02 §2).
4. Launch: release notes. Community posts (Discord, r/anno) are Nia's, if wanted.

**Verify:** a signed installer installs on a clean Windows VM, auto-updates from vN to vN+1, and launches with 0 errors.

**Estimate:** 12h with Claude | 28h without.

## Totals

| Phase                  | With Claude | Without  |
| ---------------------- | ----------- | -------- |
| 0b Validate            | 10h         | 16h      |
| 1 Foundation           | 14h         | 36h      |
| 2 Mod engine           | 28h         | 72h      |
| 3 Library UI           | 22h         | 60h      |
| 4 Installer            | 20h         | 52h      |
| 5 mod.io               | 34h         | 84h      |
| 6 Health               | 10h         | 26h      |
| 7 Onboarding/i18n/a11y | 14h         | 36h      |
| 8 Hardening/QA         | 18h         | 40h      |
| 9 Release              | 12h         | 28h      |
| **Total**              | **182h**    | **450h** |

That is a 2.5x multiplier, about 60% of the time saved. External waits are not included: mod.io OAuth approval, a code-signing certificate, and Ubisoft's answer if one is needed.

## Risks and open questions

| #   | Risk                                                                                    | Owner          | Mitigation                                                                                         |
| --- | --------------------------------------------------------------------------------------- | -------------- | -------------------------------------------------------------------------------------------------- |
| R1  | Email/platform login needs Ubisoft's game key; "Sign in with mod.io" needs an OAuth app | Angelo, Ravi   | Ship with token paste first, then device login. Request the OAuth app in Phase 0b.                 |
| R2  | A shared user API key is limited to 60 requests/min across all installs                 | Ravi           | Prefer the bearer token (120/min per user), aggressive caching, and `id-in` batched update checks. |
| R3  | Ubisoft can disable third-party API access (`api_access_options`)                       | Marcus         | Local features (library, drop install, health) work with no mod.io. Show a degraded state.         |
| R4  | The Anno 117 path, dash character and activation scheme differ between docs and builds  | Ravi           | Detect both, verify on a real install in Phase 0b, and keep fixtures for both schemes.             |
| R5  | The Anno 1800 in-game mod.io folder duplicates our installs                             | Ravi           | Read-only detection, plus a Health card. Never write there.                                        |
| R6  | RAR licence                                                                             | Angelo, Marcus | Confirm in Phase 0b. If unclear, ship zip + 7z and show a clear "rar not supported" message.       |
| R7  | Code signing cost                                                                       | Angelo, Jordan | Azure Trusted Signing (the low-cost option) decided before Phase 9.                                |
| R8  | Product name and branding must not imply Ubisoft affiliation                            | Angelo, Dex    | Working name "Atelier", with a "not affiliated with Ubisoft" line in About and the docs.           |

## Task graph (compiled when the build plan is accepted in plan mode)

```taskgraph
{
  "profile": "T3",
  "nodes": [
    { "id": "mockup-review", "kind": "human-gate", "gates": ["start"], "produces": ["approved"] },
    { "id": "p1-foundation", "kind": "slice", "owner": "jordan", "phase": "1",
      "files_own": ["package.json", "src-tauri/Cargo.toml", "src-tauri/tauri.conf.json", ".github/workflows/ci.yml", "src/ui/tokens.css"],
      "produces": ["scaffold"], "verify": "pnpm build",
      "needs": [{ "node": "mockup-review", "artifact": "approved" }] },
    { "id": "p2-engine", "kind": "slice", "owner": "ravi", "phase": "2",
      "files_own": ["src-tauri/src/games/mod.rs", "src-tauri/src/mods/mod.rs", "src-tauri/src/process.rs", "src-tauri/src/watch.rs"],
      "produces": ["engine"], "verify": "cargo test --manifest-path src-tauri/Cargo.toml mods::",
      "needs": [{ "node": "p1-foundation", "artifact": "scaffold" }] },
    { "id": "p5a-modio-client", "kind": "slice", "owner": "ravi", "phase": "5",
      "files_own": ["src-tauri/src/modio/mod.rs"],
      "produces": ["modio-client"], "verify": "cargo test --manifest-path src-tauri/Cargo.toml modio::",
      "needs": [{ "node": "p1-foundation", "artifact": "scaffold" }] },
    { "id": "p3-library-ui", "kind": "slice", "owner": "lena", "phase": "3",
      "files_own": ["src/features/library/index.tsx", "src/features/order/index.tsx", "src/features/tweaks/index.tsx"],
      "produces": ["library-ui"], "verify": "pnpm test library",
      "needs": [{ "node": "p2-engine", "artifact": "engine" }] },
    { "id": "p4-installer", "kind": "slice", "owner": "ravi", "phase": "4",
      "files_own": ["src-tauri/src/install/mod.rs", "src/features/install-preview/index.tsx"],
      "produces": ["installer"], "verify": "cargo test --manifest-path src-tauri/Cargo.toml install::",
      "needs": [{ "node": "p2-engine", "artifact": "engine" }] },
    { "id": "p6-health", "kind": "slice", "owner": "lena", "phase": "6",
      "files_own": ["src-tauri/src/health/mod.rs", "src/features/health/index.tsx"],
      "produces": ["health"], "verify": "cargo test --manifest-path src-tauri/Cargo.toml health::",
      "needs": [{ "node": "p2-engine", "artifact": "engine" }] },
    { "id": "p5b-modio-ui", "kind": "slice", "owner": "lena", "phase": "5",
      "files_own": ["src/features/browse/index.tsx", "src/features/collections/index.tsx", "src/features/downloads/index.tsx"],
      "produces": ["modio-ui"], "verify": "pnpm test browse",
      "needs": [{ "node": "p5a-modio-client", "artifact": "modio-client" }, { "node": "p3-library-ui", "artifact": "library-ui" }] },
    { "id": "merge-core", "kind": "merge", "phase": "6", "produces": ["merged"], "verify": "pnpm tauri build --debug",
      "needs": [{ "node": "p4-installer", "artifact": "installer" }, { "node": "p6-health", "artifact": "health" }, { "node": "p5b-modio-ui", "artifact": "modio-ui" }] },
    { "id": "p7-onboarding", "kind": "slice", "owner": "lena", "phase": "7",
      "files_own": ["src/features/onboarding/index.tsx", "src/features/settings/index.tsx", "src/i18n/index.ts"],
      "produces": ["onboarding"], "verify": "pnpm test onboarding",
      "needs": [{ "node": "merge-core", "artifact": "merged" }] },
    { "id": "p8-review", "kind": "verify", "owner": "maya", "verifies": ["p2-engine", "p4-installer", "p5a-modio-client", "p3-library-ui", "p5b-modio-ui", "p6-health", "p7-onboarding"], "verify": "pnpm test && cargo test --manifest-path src-tauri/Cargo.toml",
      "needs": [{ "node": "p7-onboarding", "artifact": "onboarding" }] },
    { "id": "gate", "kind": "human-gate", "gates": ["push", "pr"] },
    { "id": "ship", "kind": "close", "actions": ["push", "pr"] }
  ]
}
```

Phase 9 (release) runs after the graph closes and needs Angelo's signing decision. Kai's security pass runs with Maya's at the `p8-review` node (risk 2: auth + file writes).
