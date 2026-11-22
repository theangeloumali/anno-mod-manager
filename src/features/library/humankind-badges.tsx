// Created by The Angelo Umali.
import {
  CloudDownloadIcon,
  FolderIcon,
  LayersIcon,
  LockIcon,
  type LucideIcon,
} from "lucide-react";
import { StateBadge, type StateTone } from "@/components/app/state-badge";
import { moduleKind } from "@/data/humankind-order";
import type { Mod } from "@/data/types";

interface Chip {
  label: string;
  tone: StateTone;
  icon: LucideIcon;
}

function sourceChip(mod: Pick<Mod, "origin" | "location">): Chip {
  if (mod.location === "workshop")
    return { label: "Workshop · read-only", tone: "info", icon: LockIcon };
  if (mod.location === "game")
    return { label: "Game folder · read-only", tone: "info", icon: LockIcon };
  if (mod.location === "gamemodio")
    return { label: "Game's mod.io · read-only", tone: "info", icon: LockIcon };
  if (mod.origin === "modio")
    return { label: "mod.io", tone: "neutral", icon: CloudDownloadIcon };
  return { label: "Community", tone: "neutral", icon: FolderIcon };
}

/** A blank category means the bundle wasn't read; say so rather than guess Extension. */
function typeChip(category: string): Chip {
  if (category === "")
    return { label: "Type not verified", tone: "warning", icon: LayersIcon };
  return { label: moduleKind(category), tone: "neutral", icon: LayersIcon };
}

/** Where a Humankind mod came from and what kind of module it is. */
export function HumankindModChips({ mod }: { mod: Mod }) {
  return (
    <span className="mt-1 flex flex-wrap gap-1">
      {[sourceChip(mod), typeChip(mod.category)].map((chip) => (
        <StateBadge key={chip.label} tone={chip.tone} icon={chip.icon}>
          {chip.label}
        </StateBadge>
      ))}
    </span>
  );
}
