// Created by The Angelo Umali.
import { FolderOpenIcon, LockIcon } from "lucide-react";
import { Fragment, useState } from "react";
import type { Mod, ModLocation } from "@/data/types";
import { StateBadge } from "./state-badge";

const LOCATION_LABEL: Record<ModLocation, string> = {
  documents: "Documents",
  game: "Game folder · read-only",
  ubi: "Ubisoft .ubi · read-only",
  workshop: "Steam Workshop · read-only",
  gamemodio: "Game's mod.io · read-only",
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

/** Offers a line break after `_`, `-`, `.` and slashes, so one-token names and ModIDs break
 * at their own separators; `wrap-anywhere` on the parent stays the fallback. */
export function BreakableText({ text }: { text: string }) {
  return text.split(/(?<=[_\-./\\])/).map((part, index) => (
    <Fragment key={index}>
      {index > 0 && <wbr />}
      {part}
    </Fragment>
  ));
}

/** "[Gameplay] " before a mod's name, or nothing when its modinfo gives no category. */
export const categoryPrefix = (category: string) =>
  category ? `[${category}] ` : "";

/** The mod's own mod.io logo; gradient + initials when it has none or it fails to load. */
export function ModBanner({
  name,
  logoUrl = null,
  className = "",
}: {
  name: string;
  logoUrl?: string | null;
  className?: string;
}) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  if (logoUrl && logoUrl !== failedUrl)
    return (
      <img
        src={logoUrl}
        alt=""
        loading="lazy"
        onError={() => setFailedUrl(logoUrl)}
        className={`bg-surface object-cover ${className}`}
      />
    );
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
