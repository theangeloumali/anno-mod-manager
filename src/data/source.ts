import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { mockSource } from "./mock-source.ts";
import type {
  Account,
  AuthMethod,
  DeviceCode,
  DownloadJob,
  GameId,
  GameInfo,
  HealthCheck,
  InstallPreview,
  InstallResult,
  Issue,
  LoadOrder,
  LogEntry,
  Mod,
  ModDetail,
  ModioCollection,
  ModioPage,
  ModioQuery,
  ModOption,
  Profile,
  ReportKind,
  Settings,
} from "./types.ts";

/** Everything the UI can ask of the core. The mock and the Tauri IPC source both implement it. */
export interface DataSource {
  listGames(): Promise<GameInfo[]>;
  /** null goes back to auto-detection. */
  setGamePath(game: GameId, path: string | null): Promise<void>;
  listMods(game: GameId): Promise<Mod[]>;
  setModActive(game: GameId, modId: string, active: boolean): Promise<void>;
  uninstallMod(game: GameId, modId: string): Promise<void>;
  /** Puts back the newest backup of a mod: an uninstall or a replaced version. */
  restoreMod(game: GameId, modId: string): Promise<void>;
  listIssues(game: GameId): Promise<Issue[]>;
  loadOrder(game: GameId): Promise<LoadOrder>;
  listProfiles(game: GameId): Promise<Profile[]>;
  switchProfile(game: GameId, name: string): Promise<void>;
  saveProfile(game: GameId, name: string): Promise<void>;
  deleteProfile(game: GameId, name: string): Promise<void>;
  exportProfile(game: GameId, name: string): Promise<string>;
  listOptions(game: GameId, modId: string): Promise<ModOption[]>;
  setOption(
    game: GameId,
    modId: string,
    key: string,
    value: ModOption["value"] | null,
  ): Promise<void>;
  previewInstall(game: GameId, paths: string[]): Promise<InstallPreview>;
  confirmInstall(
    previewId: string,
    includeDowngrades: boolean,
  ): Promise<InstallResult>;
  cancelInstall(previewId: string): Promise<void>;
  browseModio(query: ModioQuery): Promise<ModioPage>;
  modioTags(game: GameId): Promise<string[]>;
  modioDetail(game: GameId, modioId: number): Promise<ModDetail>;
  listCollections(game: GameId): Promise<ModioCollection[]>;
  followCollection(
    game: GameId,
    collectionId: number,
    follow: boolean,
  ): Promise<void>;
  subscribe(game: GameId, modioId: number): Promise<void>;
  /** Resolves to the profile it created, or null where the game has no profiles. */
  installCollection(game: GameId, collectionId: number): Promise<string | null>;
  syncSubscriptions(game: GameId): Promise<void>;
  listDownloads(): Promise<DownloadJob[]>;
  retryDownload(jobId: string): Promise<void>;
  cancelDownload(jobId: string): Promise<void>;
  checkUpdates(game: GameId): Promise<void>;
  updateMod(game: GameId, modId: string): Promise<void>;
  updateAll(game: GameId): Promise<void>;
  rateMod(game: GameId, modioId: number, up: boolean): Promise<void>;
  reportMod(
    game: GameId,
    modioId: number,
    kind: ReportKind,
    summary: string,
  ): Promise<void>;
  connectivity(): Promise<boolean>;
  healthChecks(game: GameId): Promise<HealthCheck[]>;
  loaderLog(game: GameId): Promise<LogEntry[]>;
  account(): Promise<Account>;
  acceptTerms(): Promise<void>;
  setApiKey(key: string): Promise<void>;
  clearApiKey(): Promise<void>;
  /** The core only supports token sign-in; the mock still demos browser and device codes. */
  signIn(method: AuthMethod, token?: string): Promise<DeviceCode | null>;
  signOut(): Promise<void>;
  getSettings(): Promise<Settings>;
  saveSettings(settings: Settings): Promise<void>;
  launchGame(game: GameId): Promise<void>;
}

let activeSource: DataSource = mockSource;

/** Called once at startup by the Tauri entry; the browser keeps the mock. */
export function setDataSource(source: DataSource): void {
  activeSource = source;
}

const keys = {
  games: ["games"] as const,
  mods: (game: GameId) => ["mods", game] as const,
  issues: (game: GameId) => ["issues", game] as const,
  order: (game: GameId) => ["load-order", game] as const,
  profiles: (game: GameId) => ["profiles", game] as const,
  options: (game: GameId, modId: string) => ["options", game, modId] as const,
  modio: (query: ModioQuery) => ["modio", query] as const,
  tags: (game: GameId) => ["modio-tags", game] as const,
  detail: (game: GameId, modioId: number) =>
    ["modio-detail", game, modioId] as const,
  collections: (game: GameId) => ["collections", game] as const,
  downloads: ["downloads"] as const,
  health: (game: GameId) => ["health", game] as const,
  log: (game: GameId) => ["log", game] as const,
  account: ["account"] as const,
  connectivity: ["connectivity"] as const,
  settings: ["settings"] as const,
};

export const useGames = () =>
  useQuery({ queryKey: keys.games, queryFn: () => activeSource.listGames() });
export const useMods = (game: GameId) =>
  useQuery({
    queryKey: keys.mods(game),
    queryFn: () => activeSource.listMods(game),
  });
export const useIssues = (game: GameId) =>
  useQuery({
    queryKey: keys.issues(game),
    queryFn: () => activeSource.listIssues(game),
  });
export const useLoadOrder = (game: GameId) =>
  useQuery({
    queryKey: keys.order(game),
    queryFn: () => activeSource.loadOrder(game),
  });
export const useProfiles = (game: GameId) =>
  useQuery({
    queryKey: keys.profiles(game),
    queryFn: () => activeSource.listProfiles(game),
  });
export const useModOptions = (game: GameId, modId: string | null) =>
  useQuery({
    queryKey: keys.options(game, modId ?? ""),
    queryFn: () => activeSource.listOptions(game, modId ?? ""),
    enabled: modId !== null,
  });
export const useModioBrowse = (query: ModioQuery) =>
  useQuery({
    queryKey: keys.modio(query),
    queryFn: () => activeSource.browseModio(query),
  });
export const useModioTags = (game: GameId) =>
  useQuery({
    queryKey: keys.tags(game),
    queryFn: () => activeSource.modioTags(game),
  });
export const useModioDetail = (game: GameId, modioId: number | null) =>
  useQuery({
    queryKey: keys.detail(game, modioId ?? 0),
    queryFn: () => activeSource.modioDetail(game, modioId ?? 0),
    enabled: modioId !== null,
  });
export const useCollections = (game: GameId) =>
  useQuery({
    queryKey: keys.collections(game),
    queryFn: () => activeSource.listCollections(game),
  });
export const useDownloads = () =>
  useQuery({
    queryKey: keys.downloads,
    queryFn: () => activeSource.listDownloads(),
    refetchInterval: 1000,
  });
export const useHealthChecks = (game: GameId) =>
  useQuery({
    queryKey: keys.health(game),
    queryFn: () => activeSource.healthChecks(game),
  });
export const useLoaderLog = (game: GameId) =>
  useQuery({
    queryKey: keys.log(game),
    queryFn: () => activeSource.loaderLog(game),
  });
export const useAccount = () =>
  useQuery({ queryKey: keys.account, queryFn: () => activeSource.account() });
const CONNECTIVITY_POLL_MS = 30_000;
export const useConnectivity = () =>
  useQuery({
    queryKey: keys.connectivity,
    queryFn: () => activeSource.connectivity(),
    refetchInterval: CONNECTIVITY_POLL_MS,
  });
export const useSettings = () =>
  useQuery({
    queryKey: keys.settings,
    queryFn: () => activeSource.getSettings(),
  });

/** Any write can change mods, issues, order and health, so a mutation refreshes all cached reads. */
export function useAction<TArgs extends unknown[], TResult>(
  action: (source: DataSource, ...args: TArgs) => Promise<TResult>,
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (args: TArgs) => action(activeSource, ...args),
    onSuccess: () => client.invalidateQueries(),
  });
}
