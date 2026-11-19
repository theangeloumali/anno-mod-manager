// The UI ↔ core contract. Field names match the Rust structs (serde camelCase) in crates/*.

export type GameId = "anno117" | "anno1800";
export type Store = "steam" | "ubisoft" | "epic" | "manual";
/** Where the loader found the mod: Documents is ours to write; game and ubi are read-only. */
export type ModLocation = "documents" | "game" | "ubi";
export type ModOrigin = "modio" | "local";
export type Severity = "ok" | "warning" | "error";

export interface GameInfo {
  id: GameId;
  displayName: string;
  installPath: string | null;
  store: Store | null;
  modsPath: string;
  logPath: string;
  running: boolean;
  /** 117: "profile" (profile.txt + profiles/) or "legacy" (active-profile.txt); 1800: "activation-json". */
  activationScheme: "profile" | "legacy" | "activation-json";
}

export interface GameSetup {
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
  reason: "load-after" | "late" | "alphabetical";
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

export type ModOptionType = "toggle" | "enum" | "slider" | "text" | "color";

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
  | "new"
  | "update"
  | "downgrade"
  | "same"
  | "wrong-game"
  | "invalid";

export interface InstallCheck {
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
}

export type ModioSort =
  | "trending"
  | "downloads"
  | "rating"
  | "newest"
  | "updated"
  | "subscribers";

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
  subscribed: boolean;
  installedModId: string | null;
}

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
  modCount: number;
  followers: number;
  following: boolean;
}

export type DownloadStatus =
  | "queued"
  | "downloading"
  | "verifying"
  | "done"
  | "failed";

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

export type AuthMethod = "browser" | "device" | "token";

export interface Account {
  /** The user's own mod.io API key is in the OS keychain; the app never ships one. */
  hasApiKey: boolean;
  signedIn: boolean;
  username: string | null;
  expiresAt: string | null;
  termsAccepted: boolean;
}

export interface DeviceCode {
  code: string;
  verifyUrl: string;
  expiresAt: string;
}

export interface Settings {
  newModsStart: "active" | "inactive" | "profile";
  installDependencies: boolean;
  backupRetentionDays: number | null;
  updateCheck: "hourly" | "startup" | "manual";
  language: string;
  updateChannel: "stable" | "beta";
}
