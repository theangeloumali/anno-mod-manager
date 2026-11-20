// Created by The Angelo Umali.
import type { HealthCheck } from "./types.ts";

/** The core's check for the old community Anno 1800 loader (atelier-core health). */
export const LEGACY_LOADER_CHECK = "legacy-mod-loader";

/** An error that stops or breaks the game; mod.io being down or signed out never does. */
export const isLaunchBlocker = (check: HealthCheck) =>
  check.level === "error" && !check.id.startsWith("modio-");

export const hasLegacyLoader = (checks: HealthCheck[]) =>
  checks.some(
    (check) => check.id === LEGACY_LOADER_CHECK && check.level === "error",
  );
