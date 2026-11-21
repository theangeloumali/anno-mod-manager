// The UI ↔ core contract. Field names match the Rust structs (serde camelCase) in crates/*.

export type GameId = "anno117" | "anno1800" | "humankind";
export const GAME_IDS: readonly GameId[] = ["anno117", "anno1800", "humankind"];
export type Store = "steam" | "ubisoft" | "epic" | "manual";
/** Where the loader found the mod: Documents is ours to write; the rest are read-only. */
export type ModLocation = "documents" | "game" | "ubi" | "workshop";
type ModOrigin = "modio" | "local";
type Severity = "ok" | "warning" | "error";

export interface GameInfo {
  id: GameId;
  displayName: string;
  installPath: string | null;
  store: Store | null;
  modsPath: string;
  logPath: string;
  running: boolean;
  /** 117: "profile" or "legacy"; 1800: "activation-json"; Humankind: "registry-list" (Registry.xml). */
  activationScheme: "profile" | "legacy" | "activation-json" | "registry-list";
  /** 1800: the old community mod loader is in place of the game's python35.dll. */
  legacyModLoader: boolean;
}

interface GameSetup {
  requiresNewGame?: boolean;
  safeToRemove?: boolean;
  multiplayer?: boolean;
  campaign?: boolean;
}

export interface ModDependencies {
  require: string[];
  optional: string[];
  loadAfter: string[];
  incompatible: string[];
  deprecate: string[];
}

export interface Mod {
  /** Normalised ModID, as the game writes it. */
  id: string;
  game: GameId;
  folder: string;
  name: string;
  category: string;
  version: string;
  author: string | null;
  description: string | null;
  difficulty: "cheat" | "easier" | "unchanged" | "harder" | null;
  location: ModLocation;
  origin: ModOrigin;
  modioId: number | null;
  active: boolean;
  sizeBytes: number;
  setup: GameSetup;
  dependencies: ModDependencies;
  knownIssues: string[];
  hasOptions: boolean;
  parentId: string | null;
  subModIds: string[];
  update: { version: string; modfileId: number } | null;
}

export type IssueKind =
  | "missing-dependency"
  | "inactive-dependency"
  | "incompatible"
  | "deprecated"
  | "dlc"
  | "duplicate-id"
  | "load-order-cycle";

export interface Issue {
  modId: string;
  level: Exclude<Severity, "ok">;
  kind: IssueKind;
  relatedModId: string | null;
  message: string;
}

export interface LoadOrderEntry {
  position: number;
  modId: string;
  phase: "normal" | "late";
  /** "player": Humankind keeps the order the player set in Registry.xml. */
  reason: "load-after" | "late" | "alphabetical" | "player";
  after: string[];
}

export interface LoadOrder {
  entries: LoadOrderEntry[];
  cycles: string[][];
}

export interface Profile {
  name: string;
  builtIn: boolean;
  active: boolean;
}

type ModOptionType = "toggle" | "enum" | "slider" | "text" | "color";

export interface ModOption {
  key: string;
  label: string;
  hint: string | null;
  group: string | null;
  type: ModOptionType;
  value: string | number | boolean;
  defaultValue: string | number | boolean;
  values?: string[];
  min?: number;
  max?: number;
  step?: number;
}

export type InstallItemStatus =
  "new" | "update" | "downgrade" | "same" | "wrong-game" | "invalid";

interface InstallCheck {
  level: Severity;
  message: string;
}

export interface InstallPreviewItem {
  rootPath: string;
  modId: string;
  name: string;
  category: string;
  version: string;
  previousVersion: string | null;
  targetGame: GameId | null;
  status: InstallItemStatus;
  checks: InstallCheck[];
  subModIds: string[];
}

export interface InstallPreview {
  id: string;
  sourceName: string;
  items: InstallPreviewItem[];
}

export interface InstallResult {
  installed: string[];
  skipped: string[];
  /** Something that went wrong after the files landed, so the install still counts. */
  warning: string | null;
}

export type ModioSort =
  "trending" | "downloads" | "rating" | "newest" | "updated" | "subscribers";

export interface ModioQuery {
  game: GameId;
  search: string;
  tags: string[];
  sort: ModioSort;
  subscribedOnly: boolean;
  offset: number;
}

export interface ModioMod {
  id: number;
  nameId: string;
  name: string;
  summary: string;
  author: string;
  tags: string[];
  downloads: number;
  rating: number;
  sizeBytes: number;
  logoUrl: string | null;
  /** First published and last changed on mod.io, in Unix seconds (0 when unknown). */
  dateAdded: number;
  dateUpdated: number;
  subscribed: boolean;
  installedModId: string | null;
}

/** The file mod.io serves for a mod's current release. */
interface Modfile {
  id: number;
  modId: number;
  filename: string;
  version: string | null;
  filesize: number;
  md5: string;
  binaryUrl: string;
  virusStatus: number;
  /** 1 = malware found (refused), 2 = flagged but allowed. */
  virusPositive: number;
}

export interface ModDetail extends ModioMod {
  description: string;
  /** null when the mod has no live file yet. */
  modfile: Modfile | null;
  hasDependencies: boolean;
}

export type ReportKind =
  | "generic"
  | "dmca"
  | "not-working"
  | "rude-content"
  | "illegal-content"
  | "stolen-content"
  | "false-information"
  | "other";

export interface ModioPage {
  items: ModioMod[];
  total: number;
  offset: number;
}

export interface ModioCollection {
  id: number;
  name: string;
  author: string;
  summary: string;
  /** mod.io's collection object carries no count. */
  modCount: number | null;
  followers: number;
  following: boolean;
}

export type DownloadStatus =
  "queued" | "downloading" | "verifying" | "done" | "failed";

export interface DownloadJob {
  id: string;
  modioId: number;
  name: string;
  sizeBytes: number;
  progress: number;
  status: DownloadStatus;
  error: string | null;
}

export interface HealthCheck {
  id: string;
  level: Severity;
  title: string;
  detail: string;
}

export interface LogEntry {
  time: string;
  level: "info" | "warning" | "error";
  modId: string | null;
  message: string;
  source: string | null;
}

export type AuthMethod = "email" | "token";

export interface Account {
  /** The user's own mod.io API key is in the OS keychain; the app never ships one. */
  hasApiKey: boolean;
  signedIn: boolean;
  username: string | null;
  expiresAt: string | null;
  termsAccepted: boolean;
}

export type AppErrorCode =
  | "not-supported"
  | "game-running"
  | "game-not-found"
  | "no-api-key"
  | "invalid-api-key"
  | "offline"
  | "unauthorized"
  | "not-found"
  | "not-signed-in"
  | "terms-required"
  | "rate-limited"
  | "read-only"
  | "invalid-input"
  | "io"
  | "install"
  | "modio";

/** What a failing core call rejects with, from Tauri and from the mock alike. */
export interface AppError {
  code: AppErrorCode;
  message: string;
}

export interface Settings {
  newModsStart: "active" | "inactive" | "profile";
  installDependencies: boolean;
  backupRetentionDays: number | null;
  updateCheck: "hourly" | "startup" | "manual";
  language: string;
  updateChannel: "stable" | "beta";
}

/** One module a Humankind save was played with; version is null when the save didn't pin one. */
interface SaveMod {
  name: string;
  version: string | null;
}

interface GameOption {
  key: string;
  value: string;
}

export interface Save {
  path: string;
  name: string;
  build: string | null;
  /** In load order, the base game left out. */
  mods: SaveMod[];
  options: GameOption[];
  worldSize: string | null;
  empireCount: number | null;
}

interface SkippedSave {
  path: string;
  reason: string;
}

export interface SaveList {
  saves: Save[];
  skipped: SkippedSave[];
}

type ModStatus =
  | { kind: "exact" }
  | { kind: "otherVersion"; installed: string }
  | { kind: "missing" };

export type ModCheck = SaveMod & { status: ModStatus };

export type SaveInspection = Save & { checks: ModCheck[] };

export interface SaveLoadReport {
  restored: SaveMod[];
  /** Still not at the save's version: not installed, or no backup of that version. */
  missing: ModCheck[];
  registryWritten: boolean;
}

/** A change to Registry.xml's mod list, as the ModListGuard plugin logged it. Newest first. */
export interface HistoryEntry {
  time: string;
  reason: string;
  culprit: string | null;
  old: string[];
  new: string[];
}

interface LoadedPlugin {
  name: string;
  version: string;
}

export interface BepInExStatus {
  present: boolean;
  version: string | null;
  /** Only then can Atelier remove it. */
  installedByAtelier: boolean;
  /** From the last game session's log, in load order. */
  plugins: LoadedPlugin[];
  logFound: boolean;
  /** A plugin changed after the log was written, so the list may be out of date. */
  logStale: boolean;
}
