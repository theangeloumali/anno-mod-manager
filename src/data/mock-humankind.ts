// Humankind's half of the mock: Registry.xml's list, app-owned profiles, saves, history, BepInEx.
// Refusals throw CoreError with the core's codes; mock-source turns them into rejections.
import { CoreError } from "./core-error.ts";
import { enableModule, moduleKind, orderProblem } from "./humankind-order.ts";
import {
  BACKED_UP_VERSIONS,
  bepinexStatus,
  HUMANKIND_MOD_SEEDS,
  MISSING_MODULE,
  seedHistory,
  seedOrder,
  seedProfiles,
  seedSaves,
} from "./mock-humankind-seeds.ts";
import type {
  BepInExStatus,
  LoadOrder,
  Mod,
  ModCheck,
  ModLocation,
  Profile,
  SaveInspection,
  SaveLoadReport,
} from "./types.ts";

const BASE_MODULE = "project mercury";
export type BepInExKind = "atelier" | "manual" | "absent";

export interface HumankindMock {
  /** Registry.xml's list after the base game, as short names. */
  order: string[];
  registryFound: boolean;
  installFound: boolean;
  profiles: ReturnType<typeof seedProfiles>;
  history: ReturnType<typeof seedHistory>;
  saves: ReturnType<typeof seedSaves>;
  bepinex: BepInExKind;
}

export const createHumankindMock = (): HumankindMock => ({
  order: seedOrder(),
  registryFound: true,
  installFound: true,
  profiles: seedProfiles(),
  history: seedHistory(),
  saves: seedSaves(),
  bepinex: "atelier",
});

export const humankindMods = (hk: HumankindMock): Mod[] =>
  HUMANKIND_MOD_SEEDS.map((seed) => ({
    author: null,
    description: null,
    modioId: null,
    sizeBytes: 4_200_000,
    ...seed,
    game: "humankind",
    difficulty: null,
    active: hk.order.includes(seed.id),
    setup: {},
    dependencies: {
      require: [],
      optional: [],
      loadAfter: [],
      incompatible: [],
      deprecate: [],
    },
    knownIssues: [],
    hasOptions: false,
    parentId: null,
    subModIds: [],
    update: null,
  }));

/** Short names, each once, the base game and blanks dropped: the core's normalize, minus the base. */
const normalize = (order: string[]): string[] => [
  ...new Set(
    order
      .map((entry) => entry.trim().toLowerCase())
      .filter((name) => name !== "" && name !== BASE_MODULE),
  ),
];

const kindOf = (mods: Mod[]) => (module: string) =>
  moduleKind(mods.find((mod) => mod.id === module)?.category ?? "");

export const missingModules = (hk: HumankindMock, mods: Mod[]): string[] =>
  hk.registryFound
    ? hk.order.filter((module) => !mods.some((mod) => mod.id === module))
    : [];

function requireRegistry(hk: HumankindMock): void {
  if (!hk.registryFound)
    throw new CoreError(
      "not-found",
      "no Users/<id>/Registry.xml under Documents\\Humankind; start Humankind once to create it",
    );
}

function storeOrder(hk: HumankindMock, mods: Mod[], order: string[]): void {
  hk.order = order;
  for (const mod of mods) mod.active = order.includes(mod.id);
}

export function humankindLoadOrder(hk: HumankindMock): LoadOrder {
  const order = hk.registryFound ? hk.order : [];
  return {
    entries: order.map((modId, index) => ({
      position: index + 1,
      modId,
      phase: "normal",
      reason: "player",
      after: [],
    })),
    cycles: [],
  };
}

/** Returns the order as written, the base game first, as set_load_order does. */
export function writeHumankindOrder(
  hk: HumankindMock,
  mods: Mod[],
  order: string[],
): string[] {
  const normalized = normalize(order);
  const problem = orderProblem(normalized, kindOf(mods));
  if (problem) throw new CoreError("invalid-input", problem);
  const missing = normalized.filter(
    (module) => !mods.some((mod) => mod.id === module),
  );
  if (missing.length > 0)
    throw new CoreError(
      "not-found",
      `not installed: ${missing.join(", ")}; Humankind would reset the mod list to vanilla`,
    );
  requireRegistry(hk);
  storeOrder(hk, mods, normalized);
  return [BASE_MODULE, ...normalized];
}

/** On goes through the replacement rule; off drops the module. Missing modules are not checked. */
export function setHumankindModActive(
  hk: HumankindMock,
  mods: Mod[],
  modId: string,
  active: boolean,
): void {
  requireRegistry(hk);
  const order = active
    ? enableModule(hk.order, modId, kindOf(mods))
    : hk.order.filter((module) => module !== modId);
  storeOrder(hk, mods, order);
}

/** Humankind resets the whole list when a listed module is gone, so uninstall drops it first. */
export function forgetHumankindModule(
  hk: HumankindMock,
  mods: Mod[],
  modId: string,
): void {
  storeOrder(
    hk,
    mods,
    hk.order.filter((module) => module !== modId),
  );
}

const findProfile = (hk: HumankindMock, name: string) => {
  const profile = hk.profiles.find(
    (candidate) => candidate.name.toLowerCase() === name.toLowerCase(),
  );
  if (!profile) throw new CoreError("not-found", `No profile named ${name}.`);
  return profile;
};

const sameList = (left: string[], right: string[]) =>
  left.length === right.length &&
  left.every((module, index) => module === right[index]);

export const humankindProfiles = (hk: HumankindMock): Profile[] =>
  hk.profiles.map((profile) => ({
    name: profile.name,
    builtIn: false,
    active: sameList(normalize(profile.modules), hk.order),
  }));

export const applyHumankindProfile = (
  hk: HumankindMock,
  mods: Mod[],
  name: string,
): void => {
  writeHumankindOrder(hk, mods, findProfile(hk, name).modules);
};

/** Names are unique ignoring case; saving over one keeps its spelling. */
export function saveHumankindProfile(hk: HumankindMock, name: string): void {
  const existing = hk.profiles.find(
    (profile) => profile.name.toLowerCase() === name.toLowerCase(),
  );
  if (existing) existing.modules = [...hk.order];
  else hk.profiles.push({ name, modules: [...hk.order] });
}

export function deleteHumankindProfile(hk: HumankindMock, name: string): void {
  const profile = findProfile(hk, name);
  hk.profiles = hk.profiles.filter((candidate) => candidate !== profile);
}

export const exportHumankindProfile = (hk: HumankindMock, name: string) =>
  [BASE_MODULE, ...findProfile(hk, name).modules]
    .map((module) => `${module}\n`)
    .join("");

/** `2026-11-21T19:42:08.0000000Z` → `Restored 2026-11-21 19-42-08 UTC`. */
export function restoreHumankindHistory(
  hk: HumankindMock,
  index: number,
): string {
  const entry = hk.history[index];
  if (!entry) throw new CoreError("not-found", "That history entry is gone.");
  const stamp = entry.time.slice(0, 19).replace("T", " ").replaceAll(":", "-");
  const name = `Restored ${stamp} UTC`;
  hk.profiles = hk.profiles.filter((profile) => profile.name !== name);
  hk.profiles.push({ name, modules: normalize(entry.old) });
  return name;
}

const findSave = (hk: HumankindMock, path: string) => {
  const save = hk.saves.saves.find((candidate) => candidate.path === path);
  if (!save)
    throw new CoreError(
      "invalid-input",
      "Choose a save from Humankind's Save Files folder.",
    );
  return save;
};

const checkMods = (mods: Mod[], saveMods: SaveInspection["mods"]): ModCheck[] =>
  saveMods.map((saveMod) => {
    const installed = mods.find((mod) => mod.id === saveMod.name);
    if (!installed) return { ...saveMod, status: { kind: "missing" } };
    if (saveMod.version === null || installed.version === saveMod.version)
      return { ...saveMod, status: { kind: "exact" } };
    return {
      ...saveMod,
      status: { kind: "otherVersion", installed: installed.version },
    };
  });

export function inspectHumankindSave(
  hk: HumankindMock,
  mods: Mod[],
  path: string,
): SaveInspection {
  const save = findSave(hk, path);
  return { ...save, checks: checkMods(mods, save.mods) };
}

/** Puts back backed-up versions; writes the list only when every mod then matches. */
/** Mirrors `ModLocation::place` in the core, so mock refusals read like real ones. */
export const READ_ONLY_PLACE: Record<ModLocation, string> = {
  documents: "Documents mods folder",
  game: "game folder",
  ubi: "Ubisoft-managed .ubi folder",
  workshop: "Steam Workshop folder",
};

export function makeHumankindSaveLoadable(
  hk: HumankindMock,
  mods: Mod[],
  path: string,
): SaveLoadReport {
  const save = findSave(hk, path);
  requireRegistry(hk);
  // As the core: a read-only copy at the wrong version would load beside a restored one.
  const readOnly = checkMods(mods, save.mods)
    .filter((check) => check.status.kind !== "exact" && check.version)
    .map((check) => mods.find((mod) => mod.id === check.name))
    .find((mod) => mod !== undefined && mod.location !== "documents");
  if (readOnly)
    throw new CoreError(
      "read-only",
      `${readOnly.id} is in the ${READ_ONLY_PLACE[readOnly.location]}, which Atelier can't change, so the version this save was played with wasn't put back. Remove that copy first.`,
    );
  const restored = checkMods(mods, save.mods)
    .filter(
      (check) =>
        check.status.kind === "otherVersion" &&
        BACKED_UP_VERSIONS.has(`${check.name}@${check.version}`),
    )
    .map(({ name, version }) => ({ name, version }));
  for (const { name, version } of restored) {
    const mod = mods.find((candidate) => candidate.id === name);
    if (mod && version) mod.version = version;
  }
  const missing = checkMods(mods, save.mods).filter(
    (check) => check.status.kind !== "exact",
  );
  const registryWritten = missing.length === 0;
  if (registryWritten)
    storeOrder(hk, mods, normalize(save.mods.map((saveMod) => saveMod.name)));
  return { restored, missing, registryWritten };
}

export const humankindBepinex = (hk: HumankindMock): BepInExStatus =>
  bepinexStatus(hk.bepinex);

export function installHumankindBepinex(hk: HumankindMock): BepInExStatus {
  if (hk.bepinex === "manual")
    throw new CoreError(
      "invalid-input",
      "BepInEx is already installed in the game folder. Atelier leaves a copy it didn't install alone.",
    );
  hk.bepinex = "atelier";
  return humankindBepinex(hk);
}

export function uninstallHumankindBepinex(hk: HumankindMock): BepInExStatus {
  if (hk.bepinex !== "atelier")
    throw new CoreError(
      "not-found",
      "Atelier didn't install this BepInEx, so it won't remove it.",
    );
  hk.bepinex = "absent";
  return humankindBepinex(hk);
}

export { MISSING_MODULE };
