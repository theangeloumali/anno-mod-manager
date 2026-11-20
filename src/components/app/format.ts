// Created by The Angelo Umali.
import type { GameId } from "@/data/types";

const BYTES_PER_KB = 1024;
const SIZE_UNITS = ["B", "KB", "MB", "GB"];

export function formatBytes(sizeBytes: number): string {
  let size = sizeBytes;
  let unitIndex = 0;
  while (size >= BYTES_PER_KB && unitIndex < SIZE_UNITS.length - 1) {
    size /= BYTES_PER_KB;
    unitIndex += 1;
  }
  const digits = size >= 10 || unitIndex === 0 ? 0 : 1;
  return `${size.toFixed(digits)} ${SIZE_UNITS[unitIndex]}`;
}

const compactNumber = new Intl.NumberFormat("en", { notation: "compact" });

export const formatCount = (count: number) => compactNumber.format(count);

const shortDate = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  year: "numeric",
});

/** mod.io's Unix-seconds dates as "18 Nov 2026"; an unknown (0) date as "—". */
export const formatUnixDate = (seconds: number) =>
  seconds > 0 ? shortDate.format(new Date(seconds * 1000)) : "—";

export const gameShortName = (game: GameId) =>
  game === "anno117" ? "Anno 117" : "Anno 1800";

export const gameExe = (game: GameId) =>
  game === "anno117" ? "Anno117.exe" : "Anno1800.exe";

/** "Anything went wrong" text for a rejected DataSource call. */
export const errorMessage = (error: unknown) =>
  error instanceof Error ? error.message : "Something went wrong.";
