// Humankind fixtures for the mock: five modules, three saves and one unreadable, two history entries.
import type { BepInExStatus, HistoryEntry, Mod, SaveList } from "./types.ts";

export type HumankindModSeed = Pick<
  Mod,
  "id" | "name" | "category" | "version" | "location" | "origin" | "folder"
> &
  Partial<Pick<Mod, "modioId" | "author" | "description" | "sizeBytes">>;

/** Listed in Registry.xml but not installed: the case where the game resets the list. */
export const MISSING_MODULE = "historical leaders";
const DOCUMENTS = "C:\\Users\\Player\\Documents\\Humankind";
const SAVE_FILES = `${DOCUMENTS}\\Save Files`;

export const HUMANKIND_MOD_SEEDS: HumankindModSeed[] = [
  {
    id: "deepsea reloaded",
    name: "DeepSea Reloaded",
    category: "Standalone",
    version: "1.4",
    location: "workshop",
    origin: "local",
    folder: "2876543210",
    author: "Abyssal",
    description: "Ocean-heavy maps with deep trenches and new naval tiles.",
  },
  {
    id: "sigma pack",
    name: "Sigma Pack",
    category: "Conversion",
    version: "3.2",
    location: "documents",
    origin: "modio",
    folder: "mods.io/4012876/Sigma Pack",
    modioId: 4012876,
    author: "Sigma",
  },
  {
    id: "the lost dynasties",
    name: "The Lost Dynasties",
    category: "Conversion",
    version: "0.9",
    location: "game",
    origin: "local",
    folder: "The Lost Dynasties",
  },
  {
    id: "prussia",
    name: "Prussia",
    category: "Extension",
    version: "2.0",
    location: "documents",
    origin: "local",
    folder: "Prussia",
    author: "Friedrich",
  },
  {
    id: "long range nuclear missiles",
    name: "Long Range Nuclear Missiles",
    category: "",
    version: "1.1",
    location: "documents",
    origin: "modio",
    folder: "mods.io/4019302/Long Range Nuclear Missiles",
    modioId: 4019302,
  },
];

export const seedOrder = (): string[] => [
  "deepsea reloaded",
  "sigma pack",
  "prussia",
  MISSING_MODULE,
  "long range nuclear missiles",
];

export const seedProfiles = (): { name: string; modules: string[] }[] => [
  {
    name: "Huge map campaign",
    modules: ["deepsea reloaded", "sigma pack", "prussia"],
  },
  { name: "Lost Dynasties test", modules: ["the lost dynasties", "prussia"] },
];

export const seedHistory = (): HistoryEntry[] => [
  {
    time: "2026-11-21T19:42:08.0000000Z",
    reason: "reset to vanilla",
    culprit: MISSING_MODULE,
    old: [
      "project mercury",
      "deepsea reloaded",
      "sigma pack",
      "prussia",
      MISSING_MODULE,
    ],
    new: ["project mercury"],
  },
  {
    time: "2026-11-20T21:05:31.0000000Z",
    reason: "mod list changed in game",
    culprit: null,
    old: ["project mercury", "sigma pack", "prussia"],
    new: [
      "project mercury",
      "deepsea reloaded",
      "sigma pack",
      "prussia",
      MISSING_MODULE,
    ],
  },
];

export const seedSaves = (): SaveList => ({
  saves: [
    {
      path: `${SAVE_FILES}\\Pangaea Huge - Turn 386.ctr`,
      name: "Pangaea Huge - Turn 386",
      build: "1.0.27.4012",
      mods: [
        { name: "deepsea reloaded", version: "1.4" },
        { name: "sigma pack", version: "3.1" },
        { name: "prussia", version: "2.0" },
        { name: MISSING_MODULE, version: "1.0" },
      ],
      options: [{ key: "GameSpeed", value: "Endless" }],
      worldSize: "Huge",
      empireCount: 10,
    },
    {
      path: `${SAVE_FILES}\\Europe quick run.ctr`,
      name: "Europe quick run",
      build: "1.0.27.4012",
      mods: [
        { name: "sigma pack", version: "3.2" },
        { name: "prussia", version: "2.0" },
      ],
      options: [],
      worldSize: "Large",
      empireCount: 6,
    },
    {
      path: `${SAVE_FILES}\\Cloud\\Vanilla benchmark on a tiny continent with every option at default.ctr`,
      name: "Vanilla benchmark on a tiny continent with every option at default",
      build: "1.0.26.3871",
      mods: [],
      options: [],
      worldSize: "Small",
      empireCount: 4,
    },
  ],
  skipped: [
    {
      path: `${SAVE_FILES}\\Autosave 212.ctr`,
      reason:
        "isn't a readable save archive (invalid Zip archive: could not find central directory end)",
    },
  ],
});

/** Saved versions the mock can put back, as the version backups would hold them. */
export const BACKED_UP_VERSIONS = new Set(["sigma pack@3.1"]);

export const bepinexStatus = (
  kind: "atelier" | "manual" | "absent",
): BepInExStatus =>
  kind === "absent"
    ? {
        present: false,
        version: null,
        installedByAtelier: false,
        plugins: [],
        logFound: false,
        logStale: false,
      }
    : {
        present: true,
        version: "5.4.23.2",
        installedByAtelier: kind === "atelier",
        plugins: [
          { name: "ModListGuard", version: "1.0.0" },
          ...(kind === "manual"
            ? [{ name: "Humankind Camera Unlocker", version: "0.3.1" }]
            : []),
        ],
        logFound: true,
        logStale: kind === "manual",
      };
