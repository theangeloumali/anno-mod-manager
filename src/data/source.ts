import {
  type QueryClient,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useCallback, useEffect, useRef } from "react";
import { mockSource } from "./mock-source.ts";
import type {
  Account,
  AuthMethod,
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
  /** Unsubscribes on mod.io and uninstalls the mod, keeping a backup. */
  unsubscribe(game: GameId, modioId: number): Promise<void>;
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
  /** mod.io emails a one-time code to this address. */
  requestSignInCode(email: string): Promise<void>;
  /** `secret` is the emailed code or a pasted personal token, by `method`. */
  signIn(method: AuthMethod, secret: string): Promise<void>;
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

/** Downloads install in the background, so a finished one must refresh what the screens show. */
export function useRefreshWhenDownloadsFinish(): void {
  const client = useQueryClient();
  const { data: jobs } = useDownloads();
  const finishedIds = useRef<Set<string> | null>(null);
  useEffect(() => {
    if (!jobs) return;
    const done = new Set(
      jobs.filter((job) => job.status === "done").map((job) => job.id),
    );
    const previous = finishedIds.current;
    finishedIds.current = done;
    // The first poll only records what was already done before this session saw it.
    if (previous && [...done].some((id) => !previous.has(id)))
      void refreshLocalData(client);
  }, [jobs, client]);
}

const HOUR_MS = 3_600_000;
const SCHEDULED_CHECK = "scheduled-update-check";

/** Re-reads after a write, except the scheduled mod.io check: it runs on its own clock. */
const refreshLocalData = (client: QueryClient) =>
  client.invalidateQueries({
    predicate: (query) => query.queryKey[0] !== SCHEDULED_CHECK,
  });
const GAME_IDS: GameId[] = ["anno117", "anno1800"];

/** Runs the mod.io update check the Settings choice asks for: once at start, or hourly. */
export function useScheduledUpdateChecks(): void {
  const client = useQueryClient();
  const updateCheck = useSettings().data?.updateCheck;
  useQuery({
    queryKey: [SCHEDULED_CHECK, updateCheck],
    queryFn: async () => {
      // One game failing (no key, offline) must not stop the other's check.
      await Promise.allSettled(
        GAME_IDS.map((game) => activeSource.checkUpdates(game)),
      );
      await client.invalidateQueries({ queryKey: ["mods"] });
      return Date.now();
    },
    enabled: updateCheck === "startup" || updateCheck === "hourly",
    staleTime: Infinity,
    refetchInterval: updateCheck === "hourly" ? HOUR_MS : false,
    refetchOnWindowFocus: false,
  });
}

const SYNC_MIN_GAP_MS = 60_000;

/**
 * Installs subscriptions made elsewhere (mod.io's site) and removes dropped ones: at start,
 * hourly, and on opening Browse or the Library, at most once a minute. Failures stay quiet;
 * the next run tries again.
 */
export function useSubscriptionSync(view: string): void {
  const client = useQueryClient();
  const isSignedIn = useAccount().data?.signedIn ?? false;
  const lastRun = useRef(0);
  const run = useCallback(async () => {
    if (Date.now() - lastRun.current < SYNC_MIN_GAP_MS) return;
    lastRun.current = Date.now();
    await Promise.allSettled(
      GAME_IDS.map((game) => activeSource.syncSubscriptions(game)),
    );
    void refreshLocalData(client);
  }, [client]);

  useEffect(() => {
    if (!isSignedIn) return;
    void run();
    const timer = window.setInterval(() => void run(), HOUR_MS);
    return () => window.clearInterval(timer);
  }, [isSignedIn, run]);

  useEffect(() => {
    if (isSignedIn && (view === "browse" || view === "library")) void run();
  }, [isSignedIn, view, run]);
}

/** Any write can change mods, issues, order and health, so a mutation refreshes all cached reads. */
export function useAction<TArgs extends unknown[], TResult>(
  action: (source: DataSource, ...args: TArgs) => Promise<TResult>,
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (args: TArgs) => action(activeSource, ...args),
    onSuccess: () => refreshLocalData(client),
  });
}
