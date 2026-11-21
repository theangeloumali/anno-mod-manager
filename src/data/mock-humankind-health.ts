// Humankind's Health checks for the mock, worded as atelier-core's health/humankind.rs words them.
import {
  humankindBepinex,
  missingModules,
  type HumankindMock,
} from "./mock-humankind.ts";
import type { HealthCheck, Mod } from "./types.ts";

const PAGEFILE_ADVICE =
  "Set a custom pagefile of 32 GB or more in Windows (System, Advanced, Performance, Virtual memory). Players report it helps against the late-game flat-tile bug on Huge maps.";

type Level = HealthCheck["level"];
const check = (
  id: string,
  level: Level,
  title: string,
  detail = "",
): HealthCheck => ({
  id,
  level,
  title,
  detail,
});

function gameFound(installPath: string | null): HealthCheck {
  return installPath
    ? check("game-found", "ok", "Game found", installPath)
    : check(
        "game-found",
        "error",
        "Game not found",
        "No Humankind install was detected",
      );
}

function missingCheck(hk: HumankindMock, mods: Mod[]): HealthCheck {
  if (!hk.registryFound)
    return check(
      "missing-modules",
      "warning",
      "Couldn't read the mod list",
      "no Users/<id>/Registry.xml under Documents\\Humankind; start Humankind once to create it",
    );
  const missing = missingModules(hk, mods);
  return missing.length === 0
    ? check("missing-modules", "ok", "Every listed module is installed")
    : check(
        "missing-modules",
        "error",
        "The mod list names modules that aren't installed",
        `At launch the game resets the mod list to the base game. Missing: ${missing.join(", ")}`,
      );
}

function bepinexChecks(
  hk: HumankindMock,
  installPath: string | null,
): HealthCheck[] {
  const status = humankindBepinex(hk);
  const hasGuard = status.plugins.some(
    (plugin) => plugin.name === "ModListGuard",
  );
  return [
    status.present
      ? check(
          "bepinex",
          "ok",
          "BepInEx is installed",
          `${installPath ?? ""}\\BepInEx`,
        )
      : check(
          "bepinex",
          "ok",
          "BepInEx is not installed",
          "Optional. Plugins such as ModListGuard need it.",
        ),
    hasGuard
      ? check(
          "modlistguard",
          "ok",
          "ModListGuard is installed",
          "It backs up the mod list and puts it back when the game resets it.",
        )
      : check(
          "modlistguard",
          "ok",
          "ModListGuard is not installed",
          "Optional BepInEx plugin that backs up the mod list and puts it back when the game resets it.",
        ),
  ];
}

export function humankindHealthChecks(
  hk: HumankindMock,
  mods: Mod[],
  installPath: string | null,
): HealthCheck[] {
  return [
    gameFound(installPath),
    check(
      "documents-writable",
      "ok",
      "Documents\\Humankind is writable",
      "C:\\Users\\Player\\Documents\\Humankind",
    ),
    missingCheck(hk, mods),
    check(
      "duplicate-guids",
      "warning",
      "The same module is installed in more than one place",
      "4b1d7e20: Prussia in Documents/Prussia, Prussia in Workshop/3012345678",
    ),
    check("pagefile", "warning", "Pagefile is system-managed", PAGEFILE_ADVICE),
    check(
      "force-vulkan",
      "warning",
      "Steam starts Humankind with -force-vulkan",
      "It causes missing harbor and wall graphics. Remove it from the game's launch options in Steam (Steam user 84211937).",
    ),
    ...(installPath ? bepinexChecks(hk, installPath) : []),
  ];
}
