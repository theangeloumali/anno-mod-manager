// Created by The Angelo Umali.
// The DataSource inside Tauri: each method is one command in src-tauri/src/commands.
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { toCoreError } from "./core-error.ts";
import type { DataSource } from "./source.ts";
import type {
  Account,
  DownloadJob,
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
  ModOption,
  Profile,
  Settings,
} from "./types.ts";

/** Tauri maps camelCase keys here onto the commands' snake_case parameters. */
const call = <T>(command: string, args?: InvokeArgs): Promise<T> =>
  invoke<T>(command, args).catch((rejection: unknown) =>
    Promise.reject(toCoreError(rejection)),
  );

/** Native folder picker; resolves null when the user cancels. */
export const pickFolder = (): Promise<string | null> =>
  call<string | null>("plugin:dialog|open", { options: { directory: true } });

/** Native file picker for mod archives; full paths, so the core never guesses a folder. */
export const pickArchives = async (): Promise<string[]> =>
  (await call<string[] | null>("plugin:dialog|open", {
    options: {
      multiple: true,
      filters: [{ name: "Mod archives", extensions: ["zip", "7z"] }],
    },
  })) ?? [];

export const ipcSource: DataSource = {
  listGames: () => call<GameInfo[]>("list_games"),
  setGamePath: (game, path) => call("set_game_path", { game, path }),
  listMods: (game) => call<Mod[]>("list_mods", { game }),
  setModActive: (game, modId, active) =>
    call("set_mod_active", { game, modId, active }),
  uninstallMod: (game, modId) => call("uninstall_mod", { game, modId }),
  restoreMod: (game, modId) => call("restore_mod", { game, modId }),
  listIssues: (game) => call<Issue[]>("list_issues", { game }),
  loadOrder: (game) => call<LoadOrder>("load_order", { game }),
  listProfiles: (game) => call<Profile[]>("list_profiles", { game }),
  switchProfile: (game, name) => call("switch_profile", { game, name }),
  saveProfile: (game, name) => call("save_profile", { game, name }),
  deleteProfile: (game, name) => call("delete_profile", { game, name }),
  exportProfile: (game, name) => call<string>("export_profile", { game, name }),
  listOptions: (game, modId) =>
    call<ModOption[]>("list_options", { game, modId }),
  setOption: (game, modId, key, value) =>
    call("set_option", { game, modId, key, value }),
  previewInstall: (game, paths) =>
    call<InstallPreview>("preview_install", { game, paths }),
  confirmInstall: (previewId, includeDowngrades) =>
    call<InstallResult>("confirm_install", { previewId, includeDowngrades }),
  cancelInstall: (previewId) => call("cancel_install", { previewId }),
  browseModio: (query) => call<ModioPage>("browse_modio", { query }),
  modioTags: (game) => call<string[]>("modio_tags", { game }),
  modioDetail: (game, modioId) =>
    call<ModDetail>("modio_detail", { game, modioId }),
  listCollections: (game) =>
    call<ModioCollection[]>("list_collections", { game }),
  followCollection: (game, collectionId, follow) =>
    call("follow_collection", { game, collectionId, follow }),
  subscribe: (game, modioId) => call("subscribe", { game, modioId }),
  unsubscribe: (game, modioId) => call("unsubscribe", { game, modioId }),
  installCollection: (game, collectionId) =>
    call<string | null>("install_collection", { game, collectionId }),
  syncSubscriptions: (game) => call("sync_subscriptions", { game }),
  listDownloads: () => call<DownloadJob[]>("list_downloads"),
  retryDownload: (jobId) => call("retry_download", { jobId }),
  cancelDownload: (jobId) => call("cancel_download", { jobId }),
  checkUpdates: (game) => call("check_updates", { game }),
  updateMod: (game, modId) => call("update_mod", { game, modId }),
  updateAll: (game) => call("update_all", { game }),
  rateMod: (game, modioId, up) => call("rate_mod", { game, modioId, up }),
  reportMod: (game, modioId, kind, summary) =>
    call("report_mod", { game, modioId, kind, summary }),
  connectivity: () => call<boolean>("connectivity"),
  healthChecks: (game) => call<HealthCheck[]>("health_checks", { game }),
  loaderLog: (game) => call<LogEntry[]>("loader_log", { game }),
  account: () => call<Account>("account"),
  acceptTerms: () => call("accept_terms"),
  setApiKey: (key) => call("set_api_key", { key }),
  clearApiKey: () => call("clear_api_key"),
  requestSignInCode: (email) => call("request_sign_in_code", { email }),
  signIn: (method, secret) => call("sign_in", { method, secret }),
  signOut: () => call("sign_out"),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings) => call("save_settings", { settings }),
  launchGame: (game) => call("launch_game", { game }),
  removeLegacyLoader: (game) =>
    call<string | null>("remove_legacy_loader", { game }),
};
