// Created by The Angelo Umali.
// Stateful in-memory DataSource for the v2 mock-up. ModIDs and folders follow the real install (research 04).
import { CoreError } from "./core-error.ts";
import type { DataSource } from "./source.ts";
import type {
  Account,
  AppErrorCode,
  DownloadJob,
  GameId,
  GameInfo,
  HealthCheck,
  InstallPreview,
  InstallPreviewItem,
  Issue,
  LoadOrder,
  LoadOrderEntry,
  LogEntry,
  Mod,
  ModDependencies,
  ModDetail,
  ModioCollection,
  ModioMod,
  ModOption,
  Settings,
} from "./types.ts";

const MOCK_LATENCY_MS = 140;
const MODIO_PAGE_SIZE = 12;
const VERIFY_STEP_MS = 700;
const BYTES_PER_MB = 1024 * 1024;

export interface MockScenario {
  emptyLibrary: boolean;
  gameRunning: boolean;
  signedOut: boolean;
  offline: boolean;
  noApiKey: boolean;
}

/** Drop sources the "Mock-up states" menu can simulate, per game. */
export const MOCK_DROP_SOURCES: Record<
  GameId,
  { pack: string; otherGame: string }
> = {
  anno117: {
    pack: "roman-pack-v2.zip",
    otherGame: "exact-numbers-babbage.zip",
  },
  anno1800: {
    pack: "industry-pack-v3.zip",
    otherGame: "taludas-increaseharbourarea.zip",
  },
};

const megabytes = (size: number) => Math.round(size * BYTES_PER_MB);

const noDependencies = (): ModDependencies => ({
  require: [],
  optional: [],
  loadAfter: [],
  incompatible: [],
  deprecate: [],
});

type ModSeed = Pick<Mod, "id" | "name" | "category" | "version"> &
  Partial<Omit<Mod, "dependencies">> & {
    dependencies?: Partial<ModDependencies>;
  };

function createMod(game: GameId, seed: ModSeed): Mod {
  return {
    folder: `[${seed.category}] ${seed.name}`,
    author: null,
    description: null,
    difficulty: "unchanged",
    location: "documents",
    origin: "modio",
    modioId: null,
    active: true,
    sizeBytes: megabytes(1),
    setup: {},
    knownIssues: [],
    hasOptions: false,
    parentId: null,
    subModIds: [],
    update: null,
    ...seed,
    game,
    dependencies: { ...noDependencies(), ...seed.dependencies },
  };
}

const ROMAN_LIFE_FOLDER = "[Content] Roman Life - Aegyptus (Khayi)";

function seedMods117(): Mod[] {
  const game = "anno117";
  return [
    createMod(game, {
      id: "no-road-maintenance",
      name: "No Road Maintenance",
      category: "Balance",
      version: "1.2.1",
      author: "Kurila",
      description:
        "Roads cost nothing to keep up. Safe to add or remove at any time.",
      difficulty: "easier",
      modioId: 4021001,
      sizeBytes: megabytes(0.1),
      setup: { safeToRemove: true, multiplayer: true },
      update: { version: "1.3.0", modfileId: 88120 },
    }),
    createMod(game, {
      id: "taludas-increaseharbourarea",
      folder: "[Cheat] Increased Harbour Area (Taludas)",
      name: "Increased Harbour Area",
      category: "Cheat",
      version: "1.0.3",
      author: "Taludas",
      description: "Doubles the buildable quay area around every harbour.",
      difficulty: "cheat",
      location: "game",
      origin: "local",
      sizeBytes: megabytes(0.2),
      setup: { safeToRemove: true, multiplayer: false },
    }),
    createMod(game, {
      id: "rl-aegyptus-khayi",
      folder: ROMAN_LIFE_FOLDER,
      name: "Roman Life - Aegyptus",
      category: "Content",
      version: "0.4.2",
      author: "Khayi",
      description:
        "Egyptian goods, raw materials and production chains. Ships as a parent with sub-mods you can switch one by one.",
      location: "game",
      origin: "local",
      sizeBytes: megabytes(64.8),
      hasOptions: true,
      setup: { requiresNewGame: false, safeToRemove: false, campaign: true },
      dependencies: { require: ["rl-base-aegyptus-khayi"] },
      subModIds: ["rl-raw-natron-khayi", "rl-raw-alabaster-khayi"],
      knownIssues: [
        "Alabaster jars show the placeholder icon in the stock overview.",
      ],
    }),
    createMod(game, {
      id: "rl-base-aegyptus-khayi",
      folder: `${ROMAN_LIFE_FOLDER}/[Base] Roman Life Aegyptus [SubMod]`,
      name: "Roman Life Aegyptus · Base",
      category: "Shared",
      version: "0.1",
      author: "Khayi",
      description: "Shared assets every Roman Life Aegyptus module needs.",
      location: "game",
      origin: "local",
      active: false,
      sizeBytes: megabytes(12.1),
    }),
    createMod(game, {
      id: "rl-raw-natron-khayi",
      folder: `${ROMAN_LIFE_FOLDER}/[Raw] Natron [SubMod]`,
      name: "Natron",
      category: "Raw",
      version: "0.1",
      author: "Khayi",
      location: "game",
      origin: "local",
      parentId: "rl-aegyptus-khayi",
      sizeBytes: megabytes(3.4),
    }),
    createMod(game, {
      id: "rl-raw-alabaster-khayi",
      folder: `${ROMAN_LIFE_FOLDER}/[Raw] Alabaster [SubMod]`,
      name: "Alabaster",
      category: "Raw",
      version: "0.1",
      author: "Khayi",
      location: "game",
      origin: "local",
      parentId: "rl-aegyptus-khayi",
      sizeBytes: megabytes(2.9),
    }),
    createMod(game, {
      id: "harbour-expansion-cassius",
      name: "Harbour Expansion",
      category: "Gameplay",
      version: "0.9.1",
      author: "Cassius",
      description:
        "Twelve new quay and warehouse modules with a cargo-handling chain.",
      modioId: 4021006,
      sizeBytes: megabytes(42),
      setup: { requiresNewGame: true, safeToRemove: false, multiplayer: false },
      dependencies: {
        require: ["aqueduct-core-felix"],
        deprecate: ["old-harbour-fix-cassius"],
      },
      knownIssues: ["Quay modules may float on custom maps."],
    }),
    createMod(game, {
      id: "old-harbour-fix-cassius",
      name: "Old Harbour Fix",
      category: "Fix",
      version: "0.4.0",
      author: "Cassius",
      description: "Fixes harbour pathing. Folded into Harbour Expansion.",
      origin: "local",
      sizeBytes: megabytes(0.1),
      setup: { safeToRemove: true, multiplayer: true },
    }),
    createMod(game, {
      id: "legion-economy-marcus",
      name: "Legion Economy",
      category: "Balance",
      version: "1.1.0",
      author: "Marcus V.",
      description: "Military upkeep scales with population. Tighter late game.",
      difficulty: "harder",
      modioId: 4021009,
      sizeBytes: megabytes(0.4),
      setup: { safeToRemove: false, multiplayer: true },
      dependencies: { incompatible: ["relaxed-economy-junia"] },
    }),
    createMod(game, {
      id: "relaxed-economy-junia",
      name: "Relaxed Economy",
      category: "Balance",
      version: "2.0.4",
      author: "Junia",
      description: "Lower upkeep and faster construction for a calmer game.",
      difficulty: "easier",
      modioId: 4021010,
      sizeBytes: megabytes(0.3),
      setup: { safeToRemove: true, multiplayer: true },
    }),
    // Edge state: the same ModID in the game folder and in Documents; the Documents copy loads.
    createMod(game, {
      id: "camera-unlock-tiberius",
      name: "Camera Unlock",
      category: "UI",
      version: "4.0.0",
      location: "game",
      origin: "local",
      folder: "Camera Unlock (old copy)",
      description: "An older copy left in the game folder.",
      sizeBytes: megabytes(0.2),
    }),
    createMod(game, {
      id: "camera-unlock-tiberius",
      name: "Camera Unlock",
      category: "UI",
      version: "4.2.0",
      author: "Tiberius",
      description:
        "Zoom further out and tilt lower. Loads last so it wins every camera setting.",
      modioId: 4021011,
      sizeBytes: megabytes(0.2),
      hasOptions: true,
      setup: { safeToRemove: true, multiplayer: true },
      dependencies: { loadAfter: ["*"] },
    }),
    createMod(game, {
      id: "fertility-atlas-sabina",
      name: "Fertility Atlas",
      category: "Gameplay",
      version: "1.8.0",
      author: "Sabina",
      description: "Configurable island fertilities with a map overlay.",
      difficulty: "easier",
      modioId: 4021012,
      sizeBytes: megabytes(3.4),
      hasOptions: true,
      setup: { requiresNewGame: true, safeToRemove: false, multiplayer: true },
      dependencies: { loadAfter: ["taludas-increaseharbourarea"] },
    }),
    createMod(game, {
      id: "night-lighting-aurelian",
      name: "Night Lighting",
      category: "Visual",
      version: "1.0.3",
      author: "Aurelian",
      description: "Warmer lamps and torch glow at night.",
      modioId: 4021013,
      active: false,
      sizeBytes: megabytes(6.1),
      setup: { safeToRemove: true, multiplayer: true },
    }),
    createMod(game, {
      id: "ubi-legionary-banners",
      folder: ".ubi/legionary-banners",
      name: "Legionary Banners",
      category: "Cosmetic",
      version: "1.0.0",
      author: "Ubisoft Creators",
      description:
        "Installed from the in-game mod browser. Ubisoft manages this folder.",
      location: "ubi",
      origin: "local",
      sizeBytes: megabytes(8.6),
      setup: { safeToRemove: true, multiplayer: true },
    }),
  ];
}

function seedMods1800(): Mod[] {
  const game = "anno1800";
  return [
    createMod(game, {
      id: "shared-production-library-ada",
      name: "Shared Production Library",
      category: "Shared",
      version: "1.6.0",
      author: "Ada L.",
      description: "Production templates other mods reuse.",
      modioId: 2100001,
      sizeBytes: megabytes(2.1),
    }),
    createMod(game, {
      id: "industrial-zoning-brunel",
      name: "Industrial Zoning",
      category: "Gameplay",
      version: "3.1.0",
      author: "Brunel",
      description:
        "Zones that boost factories and cut pollution near residences.",
      modioId: 2100002,
      sizeBytes: megabytes(12.9),
      dependencies: { require: ["shared-production-library-ada"] },
    }),
    createMod(game, {
      id: "railway-overhaul-stephenson",
      name: "Railway Overhaul",
      category: "Gameplay",
      version: "2.0.0",
      author: "Stephenson",
      description: "Freight rail with depots, signals and coal upkeep.",
      modioId: 2100003,
      sizeBytes: megabytes(38.2),
      dependencies: { loadAfter: ["industrial-zoning-brunel"] },
      update: { version: "2.1.0", modfileId: 55102 },
    }),
    createMod(game, {
      id: "skyscraper-tiers-otis",
      name: "Skyscraper Tiers",
      category: "Gameplay",
      version: "1.2.0",
      author: "Otis",
      description: "Three more skyscraper tiers. Needs the Seat of Power DLC.",
      modioId: 2100004,
      sizeBytes: megabytes(71.5),
    }),
    createMod(game, {
      id: "pier-lighting-telford",
      name: "Pier Lighting",
      category: "Visual",
      version: "1.1.0",
      author: "Telford",
      description: "Gas lamps along every pier.",
      modioId: 2100005,
      sizeBytes: megabytes(4.2),
      dependencies: { loadAfter: ["street-lamps-davy"] },
    }),
    createMod(game, {
      id: "street-lamps-davy",
      name: "Street Lamps",
      category: "Visual",
      version: "2.3.0",
      author: "Davy",
      description: "Brighter, warmer street lamps in every district.",
      modioId: 2100006,
      sizeBytes: megabytes(5.8),
      dependencies: { loadAfter: ["pier-lighting-telford"] },
    }),
    createMod(game, {
      id: "electricity-rebalance-faraday",
      name: "Electricity Rebalance",
      category: "Balance",
      version: "0.8.2",
      author: "Faraday",
      description: "Power plants supply wider radii and cost more oil.",
      modioId: 2100007,
      sizeBytes: megabytes(0.2),
      dependencies: { incompatible: ["power-plus-edison"] },
    }),
    createMod(game, {
      id: "power-plus-edison",
      name: "Power Plus",
      category: "Balance",
      version: "1.3.0",
      author: "Edison",
      description: "Doubles power plant output.",
      origin: "local",
      active: false,
      sizeBytes: megabytes(0.3),
    }),
    createMod(game, {
      id: "exact-numbers-babbage",
      name: "Exact Numbers",
      category: "UI",
      version: "4.0.0",
      author: "Babbage",
      description: "Shows exact production rates instead of rounded ones.",
      modioId: 2100009,
      sizeBytes: megabytes(0.4),
      hasOptions: true,
      dependencies: { loadAfter: ["*"] },
    }),
  ];
}

/** DLC the mock account doesn't own; the real core reads this from modinfo DLC dependencies. */
const UNOWNED_DLC: Record<string, string> = {
  "skyscraper-tiers-otis": "Seat of Power",
};

function seedOptions(): Record<string, ModOption[]> {
  return {
    "anno117:camera-unlock-tiberius": [
      {
        key: "maxZoom",
        label: "Maximum zoom-out",
        hint: "1.0 is the game's own limit.",
        group: "Camera",
        type: "slider",
        value: 2.2,
        defaultValue: 1.5,
        min: 1,
        max: 3,
        step: 0.1,
      },
      {
        key: "minPitch",
        label: "Lowest tilt",
        hint: "Degrees above the horizon.",
        group: "Camera",
        type: "slider",
        value: 25,
        defaultValue: 45,
        min: 10,
        max: 60,
        step: 5,
      },
      {
        key: "edgeScroll",
        label: "Edge scrolling",
        hint: null,
        group: "Behaviour",
        type: "toggle",
        value: true,
        defaultValue: true,
      },
      {
        key: "smoothing",
        label: "Camera smoothing",
        hint: null,
        group: "Behaviour",
        type: "enum",
        value: "Light",
        defaultValue: "Light",
        values: ["Off", "Light", "Cinematic"],
      },
    ],
    "anno117:fertility-atlas-sabina": [
      {
        key: "preset",
        label: "Fertility preset",
        hint: "Applies to new games only.",
        group: "Fertilities",
        type: "enum",
        value: "Generous",
        defaultValue: "Vanilla",
        values: ["Vanilla", "Generous", "Every island has everything"],
      },
      {
        key: "showOverlay",
        label: "Show map overlay",
        hint: null,
        group: "Overlay",
        type: "toggle",
        value: true,
        defaultValue: false,
      },
      {
        key: "overlayColor",
        label: "Overlay colour",
        hint: null,
        group: "Overlay",
        type: "color",
        value: "#7fd9a0",
        defaultValue: "#8ea9c4",
      },
      {
        key: "overlayLabel",
        label: "Overlay label",
        hint: "Shown on the map legend.",
        group: "Overlay",
        type: "text",
        value: "Fertile",
        defaultValue: "Fertility",
      },
    ],
    "anno117:rl-aegyptus-khayi": [
      {
        key: "rl-aegyptus-khayi.rl_alabasterunguentjars",
        label: "Alabaster unguent jars",
        hint: "Adds the luxury good and its workshop.",
        group: "Sub-mods",
        type: "toggle",
        value: true,
        defaultValue: true,
      },
      {
        key: "rl-aegyptus-khayi.rl_alexandrianmosaicglass",
        label: "Alexandrian mosaic glass",
        hint: null,
        group: "Sub-mods",
        type: "toggle",
        value: false,
        defaultValue: true,
      },
    ],
    "anno1800:exact-numbers-babbage": [
      {
        key: "decimals",
        label: "Decimal places",
        hint: null,
        group: "Display",
        type: "enum",
        value: "1",
        defaultValue: "0",
        values: ["0", "1", "2"],
      },
      {
        key: "perMinute",
        label: "Show rates per minute",
        hint: "Off shows per cycle, like the base game.",
        group: "Display",
        type: "toggle",
        value: true,
        defaultValue: false,
      },
      {
        key: "highlight",
        label: "Shortage highlight",
        hint: null,
        group: "Display",
        type: "color",
        value: "#ff9c94",
        defaultValue: "#ff9c94",
      },
    ],
  };
}

const modioItem = (
  id: number,
  nameId: string,
  name: string,
  author: string,
  tags: string[],
  downloads: number,
  rating: number,
  sizeMb: number,
  summary: string,
): ModioMod => ({
  id,
  nameId,
  name,
  author,
  tags,
  downloads,
  rating,
  summary,
  sizeBytes: megabytes(sizeMb),
  // Every other mock mod has a logo, so the mock-up shows both a logo and the fallback.
  logoUrl: id % 2 === 0 ? mockLogo(name, id % 360) : null,
  subscribed: false,
  installedModId: null,
});

/** A generated stand-in for a mod.io logo thumbnail (320x180). */
function mockLogo(name: string, hue: number): string {
  const label = name.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 180">` +
    `<defs><linearGradient id="g" x2="1" y2="1">` +
    `<stop offset="0" stop-color="hsl(${hue} 45% 34%)"/>` +
    `<stop offset="1" stop-color="hsl(${(hue + 50) % 360} 50% 16%)"/>` +
    `</linearGradient></defs><rect width="320" height="180" fill="url(#g)"/>` +
    `<text x="18" y="160" font-family="sans-serif" font-size="22" font-weight="700" fill="#f3efe8">${label}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function seedModio(): Record<GameId, ModioMod[]> {
  return {
    anno117: [
      modioItem(
        4021002,
        "aqueduct-core-felix",
        "Aqueduct Core",
        "Felix",
        ["Gameplay", "Shared"],
        48210,
        96,
        9.8,
        "Aqueduct segments and water pressure that other mods build on.",
      ),
      modioItem(
        4021001,
        "no-road-maintenance",
        "No Road Maintenance",
        "Kurila",
        ["Balance"],
        121400,
        94,
        0.1,
        "Roads cost nothing to keep up.",
      ),
      modioItem(
        4021006,
        "harbour-expansion-cassius",
        "Harbour Expansion",
        "Cassius",
        ["Gameplay"],
        30980,
        88,
        42,
        "Twelve new quay and warehouse modules with a cargo-handling chain.",
      ),
      modioItem(
        4021003,
        "roads-of-latium-octavia",
        "Roads of Latium",
        "Octavia",
        ["Gameplay"],
        64300,
        91,
        18.4,
        "Paved via and milestone roads with a speed bonus per tier.",
      ),
      modioItem(
        4021004,
        "grand-forum-pack-livia",
        "Grand Forum Pack",
        "Livia",
        ["Cosmetic"],
        52000,
        97,
        96.3,
        "Forum ornaments, statues and colonnades. Purely visual.",
      ),
      modioItem(
        4021009,
        "legion-economy-marcus",
        "Legion Economy",
        "Marcus V.",
        ["Balance"],
        12800,
        79,
        0.4,
        "Military upkeep scales with population.",
      ),
      modioItem(
        4021010,
        "relaxed-economy-junia",
        "Relaxed Economy",
        "Junia",
        ["Balance"],
        40100,
        90,
        0.3,
        "Lower upkeep and faster construction for a calmer game.",
      ),
      modioItem(
        4021011,
        "camera-unlock-tiberius",
        "Camera Unlock",
        "Tiberius",
        ["UI"],
        150200,
        98,
        0.2,
        "Zoom further out and tilt lower.",
      ),
      modioItem(
        4021012,
        "fertility-atlas-sabina",
        "Fertility Atlas",
        "Sabina",
        ["Gameplay", "UI"],
        22900,
        92,
        3.4,
        "Configurable island fertilities with a map overlay.",
      ),
      modioItem(
        4021013,
        "night-lighting-aurelian",
        "Night Lighting",
        "Aurelian",
        ["Cosmetic"],
        18700,
        89,
        6.1,
        "Warmer lamps and torch glow at night.",
      ),
      modioItem(
        4021014,
        "celtic-village-set-brennus",
        "Celtic Village Set",
        "Brennus",
        ["Cosmetic", "Content"],
        9400,
        86,
        54.7,
        "Thatched houses and palisades for the northern provinces.",
      ),
      modioItem(
        4021015,
        "trade-route-planner-gaius",
        "Trade Route Planner",
        "Gaius",
        ["UI"],
        27600,
        93,
        2.8,
        "Route timing overlay with cargo forecasts.",
      ),
      modioItem(
        4021016,
        "aqueduct-roman-distribution-public-boost-dwojt",
        "Aqueduct Distribution Boost",
        "dwojt",
        ["Balance"],
        7300,
        84,
        0.1,
        "Aqueducts supply public buildings in a wider radius.",
      ),
    ],
    anno1800: [
      modioItem(
        2100001,
        "shared-production-library-ada",
        "Shared Production Library",
        "Ada L.",
        ["Shared"],
        88000,
        95,
        2.1,
        "Production templates other mods reuse.",
      ),
      modioItem(
        2100002,
        "industrial-zoning-brunel",
        "Industrial Zoning",
        "Brunel",
        ["Gameplay"],
        41200,
        90,
        12.9,
        "Zones that boost factories and cut pollution.",
      ),
      modioItem(
        2100003,
        "railway-overhaul-stephenson",
        "Railway Overhaul",
        "Stephenson",
        ["Gameplay"],
        36500,
        87,
        38.2,
        "Freight rail with depots, signals and coal upkeep.",
      ),
      modioItem(
        2100004,
        "skyscraper-tiers-otis",
        "Skyscraper Tiers",
        "Otis",
        ["Gameplay"],
        29900,
        91,
        71.5,
        "Three more skyscraper tiers.",
      ),
      modioItem(
        2100005,
        "pier-lighting-telford",
        "Pier Lighting",
        "Telford",
        ["Cosmetic"],
        8800,
        83,
        4.2,
        "Gas lamps along every pier.",
      ),
      modioItem(
        2100006,
        "street-lamps-davy",
        "Street Lamps",
        "Davy",
        ["Cosmetic"],
        11200,
        85,
        5.8,
        "Brighter street lamps in every district.",
      ),
      modioItem(
        2100009,
        "exact-numbers-babbage",
        "Exact Numbers",
        "Babbage",
        ["UI"],
        97300,
        98,
        0.4,
        "Exact production rates instead of rounded ones.",
      ),
      modioItem(
        2100010,
        "botanica-gardens-darwin",
        "Botanica Gardens",
        "Darwin",
        ["Cosmetic", "Content"],
        15600,
        92,
        33.1,
        "Ornamental gardens with a research bonus.",
      ),
      modioItem(
        2100011,
        "harbour-cranes-telford",
        "Harbour Cranes",
        "Telford",
        ["Cosmetic"],
        21000,
        88,
        22,
        "Animated steam cranes on every pier.",
      ),
    ],
  };
}

function seedCollections(): Record<GameId, ModioCollection[]> {
  return {
    anno117: [
      {
        id: 71001,
        name: "Roman Life complete",
        author: "Khayi",
        summary: "Every Roman Life module in the order the author tests them.",
        modCount: 3,
        followers: 4120,
        following: false,
      },
      {
        id: 71002,
        name: "Calm builder",
        author: "Junia",
        summary:
          "Lower upkeep, cosmetic sets and a freer camera for slow cities.",
        modCount: 4,
        followers: 2380,
        following: true,
      },
      {
        id: 71003,
        name: "Multiplayer parity pack",
        author: "Livia",
        summary:
          "Only mods marked multiplayer-safe, so every player loads the same set.",
        modCount: 3,
        followers: 990,
        following: false,
      },
    ],
    anno1800: [
      {
        id: 72001,
        name: "Industrial metropolis",
        author: "Brunel",
        summary: "Zoning, rail and production fixes for 100k+ cities.",
        modCount: 4,
        followers: 6200,
        following: false,
      },
      {
        id: 72002,
        name: "Vanilla+ quality of life",
        author: "Ada L.",
        summary: "UI and information mods that change no balance.",
        modCount: 2,
        followers: 3100,
        following: false,
      },
    ],
  };
}

interface StoredProfile {
  name: string;
  builtIn: boolean;
  /** null means "every installed mod on", which is what the game does with no profile file. */
  activeIds: string[] | null;
  /** Off in an all-on profile, like an explicit off line in its profile file. */
  offIds?: string[];
}

interface StoredJob {
  job: DownloadJob;
  startedAt: number;
  durationMs: number;
  failAtProgress: number | null;
  finishedAt: number | null;
  onDone: (() => void) | null;
}

interface StoredPreview {
  preview: InstallPreview;
  game: GameId;
}

const GAME_DETAILS: Record<
  GameId,
  Omit<GameInfo, "running"> & { exe: string }
> = {
  anno117: {
    id: "anno117",
    displayName: "Anno 117: Pax Romana",
    installPath: "D:\\SteamLibrary\\steamapps\\common\\Anno 117 - Pax Romana",
    store: "steam",
    modsPath: "C:\\Users\\Player\\Documents\\Anno 117 - Pax Romana\\mods",
    logPath:
      "C:\\Users\\Player\\Documents\\Anno 117 - Pax Romana\\log\\mod-loader.log",
    activationScheme: "profile",
    exe: "Anno117.exe",
  },
  anno1800: {
    id: "anno1800",
    displayName: "Anno 1800",
    installPath: "D:\\SteamLibrary\\steamapps\\common\\Anno 1800",
    store: "steam",
    modsPath: "C:\\Users\\Player\\Documents\\Anno 1800\\mods",
    logPath:
      "D:\\SteamLibrary\\steamapps\\common\\Anno 1800\\logs\\mod-loader.log",
    activationScheme: "activation-json",
    exe: "Anno1800.exe",
  },
};

const DEFAULT_SETTINGS: Settings = {
  newModsStart: "active",
  installDependencies: true,
  backupRetentionDays: 30,
  updateCheck: "startup",
  language: "en",
  updateChannel: "stable",
};

function seedDownloads(now: number): StoredJob[] {
  const stored = (
    job: Omit<DownloadJob, "progress">,
    startedAgoMs: number,
    durationMs: number,
    failAtProgress: number | null = null,
  ): StoredJob => ({
    job: { ...job, progress: 0 },
    startedAt: now - startedAgoMs,
    durationMs,
    failAtProgress,
    finishedAt: null,
    onDone: null,
  });
  return [
    stored(
      {
        id: "job-forum",
        modioId: 4021004,
        name: "Grand Forum Pack",
        sizeBytes: megabytes(96.3),
        status: "downloading",
        error: null,
      },
      9000,
      90000,
    ),
    stored(
      {
        id: "job-celtic",
        modioId: 4021014,
        name: "Celtic Village Set",
        sizeBytes: megabytes(54.7),
        status: "downloading",
        error: null,
      },
      30000,
      30500,
      0.42,
    ),
    stored(
      {
        id: "job-roads",
        modioId: 4021003,
        name: "Roads of Latium",
        sizeBytes: megabytes(18.4),
        status: "done",
        error: null,
      },
      600000,
      20000,
    ),
  ];
}

function createState() {
  const gamePaths: Partial<Record<GameId, string>> = {};
  return {
    scenario: {
      emptyLibrary: false,
      gameRunning: false,
      signedOut: false,
      offline: false,
      noApiKey: false,
    } as MockScenario,
    gamePaths,
    runningGame: null as GameId | null,
    mods: { anno117: seedMods117(), anno1800: seedMods1800() } as Record<
      GameId,
      Mod[]
    >,
    /** Uninstalled mods with their sub-mods, newest last, as the core's backups keep them. */
    backups: { anno117: [], anno1800: [] } as Record<GameId, Mod[][]>,
    profiles: {
      anno117: [
        { name: "default", builtIn: true, activeIds: null },
        { name: "Vanilla", builtIn: true, activeIds: [] },
        {
          name: "Campaign Run",
          builtIn: false,
          activeIds: [
            "no-road-maintenance",
            "rl-aegyptus-khayi",
            "rl-base-aegyptus-khayi",
            "rl-raw-natron-khayi",
            "camera-unlock-tiberius",
          ],
        },
        {
          name: "Multiplayer with Livia",
          builtIn: false,
          activeIds: [
            "no-road-maintenance",
            "relaxed-economy-junia",
            "camera-unlock-tiberius",
          ],
        },
      ],
      anno1800: [
        { name: "Default", builtIn: true, activeIds: null },
        { name: "Vanilla", builtIn: true, activeIds: [] },
        {
          name: "Endgame Metropolis",
          builtIn: false,
          activeIds: [
            "shared-production-library-ada",
            "industrial-zoning-brunel",
            "railway-overhaul-stephenson",
          ],
        },
      ],
    } as Record<GameId, StoredProfile[]>,
    activeProfile: { anno117: "default", anno1800: "Default" } as Record<
      GameId,
      string
    >,
    options: seedOptions(),
    account: {
      signedIn: true,
      username: "aurelia_builds",
      expiresAt: "2027-02-18T00:00:00Z",
      termsAccepted: true,
    } as Omit<Account, "hasApiKey">,
    apiKeySet: true,
    signInCodeSentTo: null as string | null,
    settings: { ...DEFAULT_SETTINGS },
    downloads: seedDownloads(Date.now()),
    previews: new Map<string, StoredPreview>(),
    modio: seedModio(),
    collections: seedCollections(),
    nextId: 1,
  };
}

let state = createState();

const respond = <T>(value: T): Promise<T> =>
  new Promise((resolve) =>
    setTimeout(() => resolve(structuredClone(value)), MOCK_LATENCY_MS),
  );

/** mod.io needs the user's own API key, or a signed-in token, before any read. */
const NO_API_KEY_MESSAGE =
  "Add your mod.io API key in Settings to browse mod.io.";
const lacksModioAccess = () => !state.apiKeySet && !state.account.signedIn;

const refuse = (
  message: string,
  code: AppErrorCode = "invalid-input",
): Promise<never> =>
  new Promise((_resolve, reject) =>
    setTimeout(() => reject(new CoreError(code, message)), MOCK_LATENCY_MS),
  );
const refuseOffline = () =>
  refuse("mod.io is unreachable. Check your connection.", "offline");
const refuseSignedOut = (action: string) =>
  refuse(`Sign in to mod.io to ${action}.`, "not-signed-in");

const exeName = (game: GameId) => GAME_DETAILS[game].exe;
const shortGameName = (game: GameId) =>
  game === "anno117" ? "Anno 117" : "Anno 1800";

/** Writes to the mods folder are blocked while that game runs (PRODUCT.md constraint). */
function writeBlockedReason(game: GameId): string | null {
  if (state.runningGame === game)
    return `${exeName(game)} is running. Close the game to change mods.`;
  return null;
}

function visibleMods(game: GameId): Mod[] {
  return state.scenario.emptyLibrary ? [] : state.mods[game];
}

function findMod(game: GameId, modId: string): Mod | undefined {
  return state.mods[game].find((mod) => mod.id === modId);
}

function computeLoadOrder(mods: Mod[]): LoadOrder {
  const activeMods = mods.filter((mod) => mod.active);
  const activeIds = new Set(activeMods.map((mod) => mod.id));
  const entries: LoadOrderEntry[] = [];
  const cycles: string[][] = [];
  const phases: LoadOrderEntry["phase"][] = ["normal", "late"];

  for (const phase of phases) {
    const phaseMods = activeMods
      .filter(
        (mod) =>
          mod.dependencies.loadAfter.includes("*") === (phase === "late"),
      )
      .sort((left, right) => left.id.localeCompare(right.id));
    const phaseIds = new Set(phaseMods.map((mod) => mod.id));
    const placed = new Set<string>();
    const visiting: string[] = [];

    const visit = (mod: Mod) => {
      if (placed.has(mod.id)) return;
      const loopStart = visiting.indexOf(mod.id);
      if (loopStart !== -1) {
        cycles.push([...visiting.slice(loopStart), mod.id]);
        return;
      }
      visiting.push(mod.id);
      for (const afterId of mod.dependencies.loadAfter) {
        const afterMod = phaseMods.find(
          (candidate) => candidate.id === afterId,
        );
        if (afterMod && phaseIds.has(afterId)) visit(afterMod);
      }
      visiting.pop();
      if (placed.has(mod.id)) return;
      placed.add(mod.id);
      const after = mod.dependencies.loadAfter.filter((afterId) =>
        activeIds.has(afterId),
      );
      entries.push({
        position: entries.length + 1,
        modId: mod.id,
        phase,
        reason:
          phase === "late"
            ? "late"
            : after.length > 0
              ? "load-after"
              : "alphabetical",
        after,
      });
    };
    phaseMods.forEach(visit);
  }
  return { entries, cycles };
}

function computeIssues(game: GameId): Issue[] {
  const mods = visibleMods(game);
  const byId = new Map(mods.map((mod) => [mod.id, mod]));
  const issues: Issue[] = [];
  for (const mod of mods.filter((candidate) => candidate.active)) {
    for (const requiredId of mod.dependencies.require) {
      const required = byId.get(requiredId);
      if (!required)
        issues.push({
          modId: mod.id,
          level: "error",
          kind: "missing-dependency",
          relatedModId: requiredId,
          message: `Needs ${requiredId}, which isn't installed.`,
        });
      else if (!required.active)
        issues.push({
          modId: mod.id,
          level: "error",
          kind: "inactive-dependency",
          relatedModId: requiredId,
          message: `Needs ${required.name}, which is installed but turned off.`,
        });
    }
    for (const otherId of mod.dependencies.incompatible) {
      const other = byId.get(otherId);
      if (other?.active)
        issues.push({
          modId: mod.id,
          level: "error",
          kind: "incompatible",
          relatedModId: otherId,
          message: `Can't run alongside ${other.name}. Turn one of them off.`,
        });
    }
    for (const replacedId of mod.dependencies.deprecate) {
      const replaced = byId.get(replacedId);
      if (replaced?.active)
        issues.push({
          modId: replacedId,
          level: "warning",
          kind: "deprecated",
          relatedModId: mod.id,
          message: `Replaced by ${mod.name}. Keeping both applies the same fix twice.`,
        });
    }
    const dlcName = UNOWNED_DLC[mod.id];
    if (dlcName)
      issues.push({
        modId: mod.id,
        level: "warning",
        kind: "dlc",
        relatedModId: null,
        message: `Needs the ${dlcName} DLC, which this account doesn't own. The game skips it.`,
      });
  }
  for (const cycle of computeLoadOrder(mods).cycles) {
    const [firstId] = cycle;
    if (!firstId) continue;
    issues.push({
      modId: firstId,
      level: "warning",
      kind: "load-order-cycle",
      relatedModId: cycle[1] ?? null,
      message: `LoadAfter loop: ${cycle.join(" → ")}. The game's order for these mods is undefined.`,
    });
  }
  return issues;
}

function startJob(
  modioId: number,
  name: string,
  sizeBytes: number,
  onDone: () => void,
): void {
  // Matches the core: a page already downloading isn't queued twice.
  const alreadyRunning = state.downloads.some(
    (entry) =>
      entry.job.modioId === modioId &&
      entry.job.status !== "done" &&
      entry.job.status !== "failed",
  );
  if (alreadyRunning) return;
  state.downloads.unshift({
    job: {
      id: `job-${state.nextId++}`,
      modioId,
      name,
      sizeBytes,
      progress: 0,
      status: "queued",
      error: null,
    },
    startedAt: Date.now(),
    durationMs: Math.min(9000, 1800 + (sizeBytes / (BYTES_PER_MB * 4)) * 1000),
    failAtProgress: null,
    finishedAt: null,
    onDone,
  });
}

function advanceDownloads(): void {
  const now = Date.now();
  for (const stored of state.downloads) {
    const { job } = stored;
    if (job.status === "done" || job.status === "failed") {
      if (job.status === "done") job.progress = 1;
      continue;
    }
    if (state.scenario.offline) {
      job.status = "failed";
      job.error =
        "You're offline. Retry once you reconnect; the download resumes where it stopped.";
      continue;
    }
    const progress = Math.min(1, (now - stored.startedAt) / stored.durationMs);
    if (stored.failAtProgress !== null && progress >= stored.failAtProgress) {
      job.progress = stored.failAtProgress;
      job.status = "failed";
      job.error = "mod.io's download server reset the connection (HTTP 503).";
      continue;
    }
    job.progress = progress;
    if (progress < 1) {
      job.status = "downloading";
      continue;
    }
    stored.finishedAt ??= now;
    if (now - stored.finishedAt < VERIFY_STEP_MS) {
      job.status = "verifying";
      continue;
    }
    job.status = "done";
    stored.onDone?.();
    stored.onDone = null;
  }
}

/** Mirrors the core: "Collection - <name>", numbered when taken, then switched to. */
function saveCollectionProfile(game: GameId, collectionName: string): string {
  const words = collectionName.replace(/[^\p{L}\p{N}_-]+/gu, " ").trim();
  const base = `Collection - ${words || "Mods"}`.slice(0, 60).trimEnd();
  const isTaken = (candidate: string) =>
    state.profiles[game].some(
      (stored) => stored.name.toLowerCase() === candidate.toLowerCase(),
    );
  let name = base;
  for (let number = 2; isTaken(name); number += 1) name = `${base} ${number}`;
  state.profiles[game].push({
    name,
    builtIn: false,
    activeIds: state.mods[game]
      .filter((mod) => mod.active)
      .map((mod) => mod.id),
  });
  state.activeProfile[game] = name;
  return name;
}

function installFromModio(game: GameId, item: ModioMod): void {
  const existing = findMod(game, item.nameId);
  if (existing) {
    if (existing.update) existing.version = existing.update.version;
    existing.update = null;
    return;
  }
  state.mods[game].push(
    createMod(game, {
      id: item.nameId,
      name: item.name,
      category: item.tags[0] ?? "Gameplay",
      version: "1.0.0",
      author: item.author,
      description: item.summary,
      modioId: item.id,
      sizeBytes: item.sizeBytes,
      active: state.settings.newModsStart !== "inactive",
      setup: { safeToRemove: true },
    }),
  );
}

function startUpdate(mod: Mod): void {
  const target = mod.update;
  if (!target) return;
  startJob(
    mod.modioId ?? 0,
    `${mod.name} ${target.version}`,
    mod.sizeBytes,
    () => {
      mod.version = target.version;
      mod.update = null;
    },
  );
}

type PackSeed = Omit<
  InstallPreviewItem,
  "status" | "previousVersion" | "checks"
> & {
  extraChecks?: InstallPreviewItem["checks"];
};

const PACK_CONTENTS: Record<string, PackSeed[]> = {
  "roman-pack-v2.zip": [
    {
      rootPath: "Aqueduct Core",
      modId: "aqueduct-core-felix",
      name: "Aqueduct Core",
      category: "Gameplay",
      version: "1.2.0",
      targetGame: "anno117",
      subModIds: ["aqueduct-core-visuals-felix"],
      extraChecks: [
        {
          level: "ok",
          message: "Fixes the missing dependency of Harbour Expansion",
        },
        { level: "ok", message: "Includes sub-mod: Aqueduct Core · Visuals" },
      ],
    },
    {
      rootPath: "No Road Maintenance",
      modId: "no-road-maintenance",
      name: "No Road Maintenance",
      category: "Balance",
      version: "1.3.0",
      targetGame: "anno117",
      subModIds: [],
    },
    {
      rootPath: "Camera Unlock",
      modId: "camera-unlock-tiberius",
      name: "Camera Unlock",
      category: "UI",
      version: "4.0.0",
      targetGame: "anno117",
      subModIds: [],
    },
  ],
  "industry-pack-v3.zip": [
    {
      rootPath: "Harbour Cranes",
      modId: "harbour-cranes-telford",
      name: "Harbour Cranes",
      category: "Cosmetic",
      version: "1.0.1",
      targetGame: "anno1800",
      subModIds: ["harbour-cranes-night-telford"],
      extraChecks: [
        {
          level: "ok",
          message: "Includes sub-mod: Harbour Cranes · Night shift",
        },
      ],
    },
    {
      rootPath: "Railway Overhaul",
      modId: "railway-overhaul-stephenson",
      name: "Railway Overhaul",
      category: "Gameplay",
      version: "2.1.0",
      targetGame: "anno1800",
      subModIds: [],
    },
    {
      rootPath: "Exact Numbers",
      modId: "exact-numbers-babbage",
      name: "Exact Numbers",
      category: "UI",
      version: "3.2.0",
      targetGame: "anno1800",
      subModIds: [],
    },
  ],
  "exact-numbers-babbage.zip": [
    {
      rootPath: "Exact Numbers",
      modId: "exact-numbers-babbage",
      name: "Exact Numbers",
      category: "UI",
      version: "5.0.0",
      targetGame: "anno1800",
      subModIds: [],
    },
  ],
  "taludas-increaseharbourarea.zip": [
    {
      rootPath: "[Cheat] Increased Harbour Area (Taludas)",
      modId: "taludas-increaseharbourarea",
      name: "Increased Harbour Area",
      category: "Cheat",
      version: "1.0.3",
      targetGame: "anno117",
      subModIds: [],
    },
  ],
};

const compareVersions = (left: string, right: string) =>
  left.localeCompare(right, undefined, { numeric: true });

const fileStem = (path: string) =>
  (path.split(/[\\/]/).pop() ?? path).replace(/\.(zip|7z|rar)$/i, "");

function genericPackSeed(path: string, game: GameId): PackSeed {
  const stem = fileStem(path);
  const modId =
    stem
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "dropped-mod";
  return {
    rootPath: stem,
    modId,
    name: stem,
    category: "Gameplay",
    version: "1.0.0",
    targetGame: game,
    subModIds: [],
  };
}

function buildPreviewItem(seed: PackSeed, game: GameId): InstallPreviewItem {
  const { extraChecks = [], ...item } = seed;
  if (item.targetGame !== game) {
    const otherGame = item.targetGame ?? game;
    return {
      ...item,
      previousVersion: null,
      status: "wrong-game",
      checks: [
        {
          level: "error",
          message: `Built for ${shortGameName(otherGame)} (modinfo "Anno": ${otherGame === "anno117" ? 8 : 7}). ${shortGameName(game)} won't load it.`,
        },
        { level: "ok", message: "Archive is valid, with no unsafe paths" },
      ],
    };
  }
  const installed = findMod(game, item.modId);
  if (!installed)
    return {
      ...item,
      previousVersion: null,
      status: "new",
      checks: [
        { level: "ok", message: `For ${shortGameName(game)}` },
        { level: "ok", message: `Installs to mods\\${item.rootPath}` },
        ...extraChecks,
      ],
    };
  const order = compareVersions(item.version, installed.version);
  if (order === 0)
    return {
      ...item,
      previousVersion: installed.version,
      status: "same",
      checks: [
        {
          level: "ok",
          message: "Already installed at this version, so it's skipped",
        },
      ],
    };
  if (order < 0)
    return {
      ...item,
      previousVersion: installed.version,
      status: "downgrade",
      checks: [
        {
          level: "warning",
          message: `Older than the installed ${installed.version}, so it's skipped`,
        },
        { level: "ok", message: "Archive is valid, with no unsafe paths" },
      ],
    };
  return {
    ...item,
    previousVersion: installed.version,
    status: "update",
    checks: [
      { level: "ok", message: "Same ModID, newer version" },
      {
        level: "ok",
        message: `Keeps its on/off state; ${installed.version} is backed up`,
      },
      ...(installed.setup.safeToRemove === false
        ? [
            {
              level: "warning" as const,
              message: "Not safe to remove from an existing save",
            },
          ]
        : []),
    ],
  };
}

function installPreviewItem(item: InstallPreviewItem, game: GameId): void {
  const targetGame = item.status === "wrong-game" ? item.targetGame : game;
  if (!targetGame) return;
  const installed = findMod(targetGame, item.modId);
  if (installed) {
    installed.version = item.version;
    if (
      installed.update &&
      compareVersions(installed.update.version, item.version) <= 0
    )
      installed.update = null;
    return;
  }
  state.mods[targetGame].push(
    createMod(targetGame, {
      id: item.modId,
      folder: item.rootPath,
      name: item.name,
      category: item.category,
      version: item.version,
      origin: "local",
      active: state.settings.newModsStart !== "inactive",
    }),
  );
}

function applyProfile(game: GameId, profile: StoredProfile): void {
  for (const mod of state.mods[game])
    mod.active =
      profile.activeIds === null
        ? !profile.offIds?.includes(mod.id)
        : profile.activeIds.includes(mod.id);
}

function buildLog(game: GameId): LogEntry[] {
  const details = GAME_DETAILS[game];
  const stamp = (seconds: number) =>
    `2026-11-19 13:54:${String(37 + seconds).padStart(2, "0")}.${String(100 + seconds * 37).slice(-3)}`;
  const entries: LogEntry[] = [
    {
      time: stamp(0),
      level: "info",
      modId: null,
      message: "Search mods...",
      source: null,
    },
    {
      time: stamp(0),
      level: "info",
      modId: null,
      message: 'Path: "<game dir>/mods/.ubi"',
      source: null,
    },
    {
      time: stamp(0),
      level: "info",
      modId: null,
      message: 'Path: "<game dir>/mods" (enabled by default)',
      source: null,
    },
    {
      time: stamp(1),
      level: "info",
      modId: null,
      message: `Path: "${details.modsPath.replaceAll("\\", "/")}" (enabled by default)`,
      source: null,
    },
  ];
  visibleMods(game)
    .filter((mod) => mod.active)
    .forEach((mod, index) => {
      entries.push({
        time: stamp(1 + Math.floor(index / 4)),
        level: "info",
        modId: mod.id,
        message: `Enabled mod: ${mod.folder} (${mod.id}@${mod.version})`,
        source: null,
      });
    });
  for (const issue of computeIssues(game)) {
    if (
      issue.kind === "missing-dependency" ||
      issue.kind === "inactive-dependency"
    )
      entries.push({
        time: stamp(9),
        level: "error",
        modId: issue.modId,
        message: `Dependency "${issue.relatedModId}" is not loaded; mod skipped`,
        source: null,
      });
    if (issue.kind === "dlc")
      entries.push({
        time: stamp(9),
        level: "error",
        modId: issue.modId,
        message: "Required DLC is not owned; mod skipped",
        source: null,
      });
  }
  if (game === "anno117" && !state.scenario.emptyLibrary)
    entries.push({
      time: stamp(14),
      level: "warning",
      modId: "fertility-atlas-sabina",
      message: `No matching node for Path "//Asset[Values/Standard/GUID='29526']/Values/Maintenances/Item[Product='1010017']"`,
      source: "data/base/config/export/assets.xml:8",
    });
  if (game === "anno1800" && !state.scenario.emptyLibrary)
    entries.push({
      time: stamp(14),
      level: "warning",
      modId: "pier-lighting-telford",
      message:
        "LoadAfter cycle with street-lamps-davy; using alphabetical order",
      source: null,
    });
  return entries;
}

function buildHealthChecks(game: GameId): HealthCheck[] {
  const details = GAME_DETAILS[game];
  const blocked = writeBlockedReason(game);
  const loadedCount = visibleMods(game).filter((mod) => mod.active).length;
  return [
    {
      id: "game-detected",
      level: "ok",
      title: `${details.displayName} found`,
      detail: `Steam · ${details.installPath}`,
    },
    blocked
      ? {
          id: "mods-folder",
          level: "error",
          title: "Mods folder is locked",
          detail: blocked,
        }
      : {
          id: "mods-folder",
          level: "ok",
          title: "Mods folder is writable",
          detail: details.modsPath,
        },
    {
      id: "mod-loader",
      level: "ok",
      title: "Mod loader is active",
      detail: `mod-loader.log written 2 minutes ago · ${loadedCount} mods loaded`,
    },
    state.scenario.offline
      ? {
          id: "modio-reachable",
          level: "error",
          title: "mod.io is unreachable",
          detail:
            "Installed mods still work. Browsing, subscriptions and updates come back when you reconnect.",
        }
      : {
          id: "modio-reachable",
          level: "ok",
          title: "mod.io is reachable",
          detail: "g-anno.modapi.io answered in 84 ms",
        },
    state.account.signedIn
      ? {
          id: "modio-account",
          level: "ok",
          title: "Signed in to mod.io",
          detail: `${state.account.username} · token in Windows Credential Manager`,
        }
      : {
          id: "modio-account",
          level: "warning",
          title: "Not signed in to mod.io",
          detail:
            "Browsing and installing work as a guest. Sign in to subscribe, rate and sync.",
        },
    {
      id: "backups",
      level: "ok",
      title: "Backups are on",
      detail: `Replaced versions are kept for ${state.settings.backupRetentionDays ?? "∞"} days`,
    },
  ];
}

/** Moves a mod and its sub-mods into the backups, as Uninstall does. */
function removeToBackups(game: GameId, modId: string): void {
  const isRemoved = (candidate: Mod) =>
    candidate.id === modId || candidate.parentId === modId;
  state.backups[game].push(state.mods[game].filter(isRemoved));
  state.mods[game] = state.mods[game].filter(
    (candidate) => !isRemoved(candidate),
  );
}

export function mockScenario(): MockScenario {
  return { ...state.scenario, gameRunning: state.runningGame !== null };
}

/** Flips a "Mock-up states" switch; the caller refreshes React Query afterwards. */
export function setMockScenario(
  patch: Partial<MockScenario>,
  game: GameId,
): void {
  state.scenario = { ...state.scenario, ...patch };
  if (patch.gameRunning !== undefined)
    state.runningGame = patch.gameRunning ? game : null;
  if (patch.signedOut === true) {
    state.signInCodeSentTo = null;
    state.account = {
      signedIn: false,
      username: null,
      expiresAt: null,
      termsAccepted: false,
    };
  }
  if (patch.noApiKey !== undefined) state.apiKeySet = !patch.noApiKey;
  if (patch.signedOut === false && !state.account.signedIn)
    state.account = {
      signedIn: true,
      username: "aurelia_builds",
      expiresAt: "2027-02-18T00:00:00Z",
      termsAccepted: true,
    };
}

export function restartSampleDownloads(): void {
  state.downloads = seedDownloads(Date.now());
}

export function resetMockData(): void {
  state = createState();
}

export const mockSource: DataSource = {
  listGames: () =>
    respond(
      (Object.keys(GAME_DETAILS) as GameId[]).map((game) => {
        const { exe: _exe, ...info } = GAME_DETAILS[game];
        const chosenPath = state.gamePaths[game];
        return {
          ...info,
          ...(chosenPath && { installPath: chosenPath, store: "manual" }),
          running: state.runningGame === game,
        };
      }),
    ),

  setGamePath: (game, path) => {
    if (path === null) delete state.gamePaths[game];
    else if (!path.trim())
      return refuse(
        `${exeName(game)} isn't in that folder. Pick the folder ${GAME_DETAILS[game].displayName} is installed in.`,
      );
    else state.gamePaths[game] = path.trim();
    return respond(undefined);
  },

  listMods: (game) => {
    advanceDownloads();
    return respond(visibleMods(game));
  },

  setModActive: (game, modId, active) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const mod = findMod(game, modId);
    if (!mod) return refuse(`No mod with ID ${modId}.`, "not-found");
    mod.active = active;
    return respond(undefined);
  },

  uninstallMod: (game, modId) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const mod = findMod(game, modId);
    if (!mod) return refuse(`No mod with ID ${modId}.`, "not-found");
    if (mod.location !== "documents")
      return refuse(
        `${mod.name} lives in the ${mod.location === "ubi" ? "Ubisoft-managed .ubi" : "game"} folder. Atelier only removes mods in Documents.`,
        "read-only",
      );
    removeToBackups(game, modId);
    return respond(undefined);
  },

  restoreMod: (game, modId) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const backups = state.backups[game];
    const index = backups.findLastIndex((saved) => saved[0]?.id === modId);
    if (index === -1)
      return refuse(`There is no backup of ${modId} to restore.`, "not-found");
    const restored = backups.splice(index, 1).flat();
    state.mods[game] = [
      ...state.mods[game].filter((candidate) => candidate.id !== modId),
      ...restored,
    ];
    return respond(undefined);
  },

  listIssues: (game) => respond(computeIssues(game)),

  loadOrder: (game) => respond(computeLoadOrder(visibleMods(game))),

  listProfiles: (game) =>
    respond(
      state.profiles[game].map((profile) => ({
        name: profile.name,
        builtIn: profile.builtIn,
        active: profile.name === state.activeProfile[game],
      })),
    ),

  switchProfile: (game, name) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const target = state.profiles[game].find(
      (profile) => profile.name === name,
    );
    const current = state.profiles[game].find(
      (profile) => profile.name === state.activeProfile[game],
    );
    if (!target) return refuse(`No profile named ${name}.`, "not-found");
    if (current && !current.builtIn)
      current.activeIds = state.mods[game]
        .filter((mod) => mod.active)
        .map((mod) => mod.id);
    state.activeProfile[game] = name;
    applyProfile(game, target);
    return respond(undefined);
  },

  saveProfile: (game, name) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const activeIds = state.mods[game]
      .filter((mod) => mod.active)
      .map((mod) => mod.id);
    const existing = state.profiles[game].find(
      (profile) => profile.name === name,
    );
    if (existing?.builtIn)
      return refuse(`${name} is built in. Save under a new name.`, "read-only");
    if (existing) existing.activeIds = activeIds;
    else state.profiles[game].push({ name, builtIn: false, activeIds });
    state.activeProfile[game] = name;
    return respond(undefined);
  },

  deleteProfile: (game, name) => {
    const profile = state.profiles[game].find(
      (candidate) => candidate.name === name,
    );
    if (!profile) return refuse(`No profile named ${name}.`, "not-found");
    if (profile.builtIn)
      return refuse(`${name} is built in and can't be deleted.`, "read-only");
    state.profiles[game] = state.profiles[game].filter(
      (candidate) => candidate.name !== name,
    );
    if (state.activeProfile[game] === name)
      state.activeProfile[game] = state.profiles[game][0]?.name ?? "default";
    return respond(undefined);
  },

  exportProfile: (game, name) => {
    const profile = state.profiles[game].find(
      (candidate) => candidate.name === name,
    );
    if (!profile) return refuse(`No profile named ${name}.`, "not-found");
    const ids =
      profile.activeIds ??
      state.mods[game].filter((mod) => mod.active).map((mod) => mod.id);
    return respond(
      [`# Atelier profile · ${shortGameName(game)} · ${name}`, ...ids].join(
        "\n",
      ),
    );
  },

  listOptions: (game, modId) =>
    respond(state.options[`${game}:${modId}`] ?? []),

  setOption: (game, modId, key, value) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const mod = findMod(game, modId);
    if (mod && mod.location !== "documents")
      return refuse(
        `${mod.name} lives in the game folder, so its options are read-only here.`,
        "read-only",
      );
    const option = state.options[`${game}:${modId}`]?.find(
      (candidate) => candidate.key === key,
    );
    if (!option) return refuse(`No option ${key} on ${modId}.`, "not-found");
    option.value = value ?? option.defaultValue;
    return respond(undefined);
  },

  previewInstall: (game, paths) => {
    const seeds = paths.flatMap(
      (path) =>
        PACK_CONTENTS[path.split(/[\\/]/).pop() ?? path] ?? [
          genericPackSeed(path, game),
        ],
    );
    const preview: InstallPreview = {
      id: `preview-${state.nextId++}`,
      sourceName: paths
        .map((path) => path.split(/[\\/]/).pop() ?? path)
        .join(", "),
      items: seeds.map((seed) => buildPreviewItem(seed, game)),
    };
    state.previews.set(preview.id, { preview, game });
    return respond(preview);
  },

  confirmInstall: (previewId, includeDowngrades) => {
    const stored = state.previews.get(previewId);
    if (!stored)
      return refuse(
        "That preview has expired. Drop the file again.",
        "not-found",
      );
    const onlyWrongGame = stored.preview.items.every(
      (item) => item.status === "wrong-game",
    );
    const installable = stored.preview.items.filter(
      (item) =>
        item.status === "new" ||
        item.status === "update" ||
        (item.status === "downgrade" && includeDowngrades) ||
        (item.status === "wrong-game" && onlyWrongGame),
    );
    const targetGame = onlyWrongGame
      ? (installable[0]?.targetGame ?? stored.game)
      : stored.game;
    const blocked = writeBlockedReason(targetGame);
    if (blocked) return refuse(blocked, "game-running");
    installable.forEach((item) => installPreviewItem(item, stored.game));
    state.previews.delete(previewId);
    const installedIds = new Set(installable.map((item) => item.modId));
    return respond({
      installed: [...installedIds],
      skipped: stored.preview.items
        .filter((item) => !installedIds.has(item.modId))
        .map((item) => item.modId),
      warning: null,
    });
  },

  cancelInstall: (previewId) => {
    state.previews.delete(previewId);
    return respond(undefined);
  },

  browseModio: (query) => {
    if (state.scenario.offline) return refuseOffline();
    if (lacksModioAccess()) return refuse(NO_API_KEY_MESSAGE, "no-api-key");
    const installedIds = new Set(state.mods[query.game].map((mod) => mod.id));
    const search = query.search.trim().toLowerCase();
    const sortValue: Record<string, (item: ModioMod) => number> = {
      trending: (item) => item.downloads * item.rating,
      downloads: (item) => item.downloads,
      rating: (item) => item.rating,
      newest: (item) => item.id,
      updated: (item) => item.id % 7,
      subscribers: (item) => item.downloads / 3,
    };
    const score = sortValue[query.sort] ?? ((item: ModioMod) => item.downloads);
    const matches = state.modio[query.game]
      .map((item) => ({
        ...item,
        installedModId: installedIds.has(item.nameId) ? item.nameId : null,
      }))
      .filter(
        (item) =>
          !search ||
          `${item.name} ${item.nameId} ${item.summary} ${item.author}`
            .toLowerCase()
            .includes(search),
      )
      .filter((item) => query.tags.every((tag) => item.tags.includes(tag)))
      .filter((item) => !query.subscribedOnly || item.subscribed)
      .sort((left, right) => score(right) - score(left));
    return respond({
      items: matches.slice(query.offset, query.offset + MODIO_PAGE_SIZE),
      total: matches.length,
      offset: query.offset,
    });
  },

  modioTags: (game) =>
    respond(
      [...new Set(state.modio[game].flatMap((item) => item.tags))].sort(),
    ),

  modioDetail: (game, modioId) => {
    if (state.scenario.offline) return refuseOffline();
    if (lacksModioAccess()) return refuse(NO_API_KEY_MESSAGE, "no-api-key");
    const item = state.modio[game].find(
      (candidate) => candidate.id === modioId,
    );
    if (!item)
      return refuse(
        `mod.io has no mod ${modioId} for ${shortGameName(game)}.`,
        "not-found",
      );
    const detail: ModDetail = {
      ...item,
      installedModId: findMod(game, item.nameId)?.id ?? null,
      description: `${item.summary}\n\nTested on the current ${shortGameName(game)} patch.`,
      modfile: {
        id: item.id * 10,
        modId: item.id,
        filename: `${item.nameId}.zip`,
        version: "1.0.0",
        filesize: item.sizeBytes,
        md5: "9e107d9d372bb6826bd81d3542a419d6",
        binaryUrl: `https://g-anno.modapi.io/v1/games/mods/${item.id}/files/${item.id * 10}/download`,
        virusStatus: 1,
        virusPositive: 0,
      },
      hasDependencies: false,
    };
    return respond(detail);
  },

  listCollections: (game) => {
    if (state.scenario.offline) return refuseOffline();
    if (lacksModioAccess()) return refuse(NO_API_KEY_MESSAGE, "no-api-key");
    return respond(state.collections[game]);
  },

  followCollection: (game, collectionId, follow) => {
    if (state.scenario.offline) return refuseOffline();
    if (!state.account.signedIn) return refuseSignedOut("follow collections");
    const collection = state.collections[game].find(
      (candidate) => candidate.id === collectionId,
    );
    if (!collection)
      return refuse("That collection no longer exists.", "not-found");
    if (collection.following !== follow)
      collection.followers += follow ? 1 : -1;
    collection.following = follow;
    return respond(undefined);
  },

  subscribe: (game, modioId) => {
    if (state.scenario.offline) return refuseOffline();
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const item = state.modio[game].find(
      (candidate) => candidate.id === modioId,
    );
    if (!item)
      return refuse(
        `mod.io has no mod ${modioId} for ${shortGameName(game)}.`,
        "not-found",
      );
    if (state.account.signedIn) item.subscribed = true;
    // Signed in, an installed copy is kept; subscribing only adds it to the sync.
    if (!(state.account.signedIn && findMod(game, item.nameId)))
      startJob(item.id, item.name, item.sizeBytes, () =>
        installFromModio(game, item),
      );
    return respond(undefined);
  },

  unsubscribe: (game, modioId) => {
    if (!state.account.signedIn) return refuseSignedOut("unsubscribe");
    if (state.scenario.offline) return refuseOffline();
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const item = state.modio[game].find(
      (candidate) => candidate.id === modioId,
    );
    if (!item)
      return refuse(
        `mod.io has no mod ${modioId} for ${shortGameName(game)}.`,
        "not-found",
      );
    item.subscribed = false;
    const installed = findMod(game, item.nameId);
    if (installed?.location === "documents")
      removeToBackups(game, installed.id);
    return respond(undefined);
  },

  installCollection: (game, collectionId) => {
    if (state.scenario.offline) return refuseOffline();
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const collection = state.collections[game].find(
      (candidate) => candidate.id === collectionId,
    );
    if (!collection)
      return refuse("That collection no longer exists.", "not-found");
    const installedIds = new Set(state.mods[game].map((mod) => mod.id));
    const picks = state.modio[game]
      .filter((item) => !installedIds.has(item.nameId))
      .slice(0, collection.modCount ?? undefined);
    const profile =
      GAME_DETAILS[game].activationScheme === "profile"
        ? saveCollectionProfile(game, collection.name)
        : null;
    // The way back: new mods stay off elsewhere. Saved lists already leave them out.
    if (profile)
      for (const stored of state.profiles[game])
        if (stored.name !== profile && stored.activeIds === null)
          stored.offIds = [
            ...(stored.offIds ?? []),
            ...picks.map((item) => item.nameId),
          ];
    picks.forEach((item) =>
      startJob(item.id, item.name, item.sizeBytes, () => {
        installFromModio(game, item);
        // Like the core's enableNewMods: a mod that lands later is on in the collection profile.
        state.profiles[game]
          .find((stored) => stored.name === profile)
          ?.activeIds?.push(item.nameId);
      }),
    );
    return respond(profile);
  },

  listDownloads: () => {
    advanceDownloads();
    return respond(state.downloads.map((stored) => stored.job));
  },

  retryDownload: (jobId) => {
    if (state.scenario.offline)
      return refuse("Still offline. Retry once you reconnect.", "offline");
    const stored = state.downloads.find(
      (candidate) => candidate.job.id === jobId,
    );
    if (!stored) return refuse("That download is gone.", "not-found");
    const resumedFrom = stored.job.progress;
    stored.job.status = "downloading";
    stored.job.error = null;
    stored.failAtProgress = null;
    stored.finishedAt = null;
    stored.durationMs = 4000;
    stored.startedAt = Date.now() - resumedFrom * stored.durationMs;
    return respond(undefined);
  },

  cancelDownload: (jobId) => {
    // Statuses move only when advanced, so catch up before judging this one.
    advanceDownloads();
    const installing = state.downloads.some(
      (candidate) =>
        candidate.job.id === jobId && candidate.job.status === "verifying",
    );
    if (installing)
      return refuse("It's already installing, so it can't be cancelled now.");
    state.downloads = state.downloads.filter(
      (candidate) => candidate.job.id !== jobId,
    );
    return respond(undefined);
  },

  updateAll: (game) => {
    if (state.scenario.offline) return refuseOffline();
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    state.mods[game]
      .filter((mod) => mod.location === "documents")
      .forEach(startUpdate);
    return respond(undefined);
  },

  updateMod: (game, modId) => {
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const mod = findMod(game, modId);
    if (!mod?.modioId)
      return refuse(
        `${modId} wasn't installed from mod.io, so it can't be updated here.`,
        "not-found",
      );
    startUpdate(mod);
    return respond(undefined);
  },

  checkUpdates: () => {
    if (state.scenario.offline) return refuseOffline();
    return respond(undefined);
  },

  syncSubscriptions: (game) => {
    if (!state.account.signedIn)
      return refuseSignedOut("sync your subscriptions");
    if (state.scenario.offline) return refuseOffline();
    const blocked = writeBlockedReason(game);
    if (blocked) return refuse(blocked, "game-running");
    const installedIds = new Set(state.mods[game].map((mod) => mod.id));
    const queuedIds = new Set(
      state.downloads
        .filter((stored) => stored.job.status !== "done")
        .map((stored) => stored.job.modioId),
    );
    state.modio[game]
      .filter(
        (item) =>
          item.subscribed &&
          !installedIds.has(item.nameId) &&
          !queuedIds.has(item.id),
      )
      .forEach((item) =>
        startJob(item.id, item.name, item.sizeBytes, () =>
          installFromModio(game, item),
        ),
      );
    return respond(undefined);
  },

  rateMod: () => {
    if (state.scenario.offline) return refuseOffline();
    if (!state.account.signedIn) return refuseSignedOut("rate mods");
    return respond(undefined);
  },

  reportMod: (_game, _modioId, _kind, summary) => {
    if (!summary.trim())
      return refuse("Say what's wrong so mod.io can act on the report.");
    if (state.scenario.offline) return refuseOffline();
    if (lacksModioAccess()) return refuse(NO_API_KEY_MESSAGE, "no-api-key");
    return respond(undefined);
  },

  connectivity: () => respond(!state.scenario.offline),

  healthChecks: (game) => respond(buildHealthChecks(game)),

  loaderLog: (game) => respond(buildLog(game)),

  account: () => respond({ ...state.account, hasApiKey: state.apiKeySet }),

  acceptTerms: () => {
    state.account.termsAccepted = true;
    return respond(undefined);
  },

  setApiKey: (key) => {
    if (state.scenario.offline)
      return refuse(
        "mod.io is unreachable, so the key can't be checked yet.",
        "offline",
      );
    // mod.io API keys are 32 hex characters; the real core checks them with GET /games/{id}.
    if (!/^[0-9a-f]{32}$/i.test(key.trim()))
      return refuse(
        "mod.io doesn't recognise that key. Copy it again from mod.io/me/access.",
        "invalid-api-key",
      );
    state.apiKeySet = true;
    state.scenario.noApiKey = false;
    return respond(undefined);
  },

  clearApiKey: () => {
    state.apiKeySet = false;
    state.scenario.noApiKey = true;
    return respond(undefined);
  },

  requestSignInCode: (email) => {
    if (state.scenario.offline) return refuseOffline();
    if (!state.account.termsAccepted)
      return refuse("Accept mod.io's terms first.", "terms-required");
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim()))
      return refuse("That isn't a valid email address.");
    state.signInCodeSentTo = email.trim();
    return respond(undefined);
  },

  signIn: (method, secret) => {
    if (state.scenario.offline) return refuseOffline();
    if (!state.account.termsAccepted)
      return refuse("Accept mod.io's terms first.", "terms-required");
    // Any 5-character code works in the mock-up once one was "emailed".
    const accepted =
      method === "email"
        ? state.signInCodeSentTo !== null &&
          /^[A-Z0-9]{5}$/i.test(secret.trim())
        : secret.trim().length >= 12;
    if (!accepted)
      return refuse(
        method === "email"
          ? "That code isn't right. Check the latest email from mod.io."
          : "mod.io rejected that token. Create a read+write token at mod.io/me/access.",
        method === "email" ? "invalid-input" : "unauthorized",
      );
    state.signInCodeSentTo = null;
    state.account = {
      signedIn: true,
      username: "aurelia_builds",
      expiresAt: "2027-02-18T00:00:00Z",
      termsAccepted: true,
    };
    return respond(undefined);
  },

  signOut: () => {
    state.signInCodeSentTo = null;
    state.account = {
      ...state.account,
      signedIn: false,
      username: null,
      expiresAt: null,
    };
    return respond(undefined);
  },

  getSettings: () => respond(state.settings),

  saveSettings: (settings) => {
    state.settings = { ...settings };
    return respond(undefined);
  },

  launchGame: (game) => {
    if (state.runningGame === game)
      return refuse(`${exeName(game)} is already running.`, "game-running");
    state.runningGame = game;
    return respond(undefined);
  },
};
