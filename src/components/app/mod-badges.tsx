// Created by The Angelo Umali.
import { FolderOpenIcon, LockIcon } from "lucide-react";
import type { Mod, ModLocation } from "@/data/types";
import { StateBadge } from "./state-badge";

const LOCATION_LABEL: Record<ModLocation, string> = {
  documents: "Documents",
  game: "Game folder · read-only",
  ubi: "Ubisoft .ubi · read-only",
};

export const isReadOnly = (mod: Pick<Mod, "location">) =>
  mod.location !== "documents";

export function LocationBadge({ location }: { location: ModLocation }) {
  return location === "documents" ? (
    <StateBadge tone="neutral" icon={FolderOpenIcon}>
      {LOCATION_LABEL.documents}
    </StateBadge>
  ) : (
    <StateBadge tone="info" icon={LockIcon}>
      {LOCATION_LABEL[location]}
    </StateBadge>
  );
}

/** Gradient + initials stand in for mod.io logos; no Anno art (PRODUCT.md). */
export function ModBanner({
  name,
  className = "",
}: {
  name: string;
  className?: string;
}) {
  const initials = name
    .split(/\s+/)
    .filter((word) => /^[A-Za-z0-9]/.test(word))
    .slice(0, 2)
    .map((word) => word.charAt(0).toUpperCase())
    .join("");
  return (
    <div
      aria-hidden="true"
      className={`grid place-items-center bg-[linear-gradient(135deg,var(--color-surface),color-mix(in_oklab,var(--color-accent)_32%,var(--color-surface)))] font-display font-bold text-accent-strong ${className}`}
    >
      {initials}
    </div>
  );
}

/** mod.io's attribution, required wherever its content is shown (research 02). */
export function PoweredByModio() {
  return (
    <span className="text-label font-semibold text-copy-dim">
      Powered by mod.io
    </span>
  );
}
