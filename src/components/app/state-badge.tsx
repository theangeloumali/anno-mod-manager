// Created by The Angelo Umali.
import {
  CircleCheckIcon,
  CircleIcon,
  InfoIcon,
  OctagonXIcon,
  TriangleAlertIcon,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export type StateTone = "ok" | "warning" | "error" | "info" | "neutral";

const TONE_CLASS: Record<StateTone, string> = {
  ok: "border-success/50 text-success",
  warning: "border-line-strong text-copy",
  error: "border-danger/60 text-danger",
  info: "border-accent/60 text-accent",
  neutral: "border-line text-copy-muted",
};

const TONE_ICON: Record<StateTone, LucideIcon> = {
  ok: CircleCheckIcon,
  warning: TriangleAlertIcon,
  error: OctagonXIcon,
  info: InfoIcon,
  neutral: CircleIcon,
};

/** A state is always icon + words, never colour alone (WCAG 1.4.1). */
export function StateBadge({
  tone,
  icon,
  children,
  className,
}: {
  tone: StateTone;
  icon?: LucideIcon;
  children: ReactNode;
  className?: string;
}) {
  const Icon = icon ?? TONE_ICON[tone];
  return (
    <Badge
      variant="outline"
      className={cn("h-auto py-0.5 text-label", TONE_CLASS[tone], className)}
    >
      <Icon aria-hidden="true" />
      {children}
    </Badge>
  );
}

/** Inline icon for check lists, matching the badge tones. */
export function StateIcon({
  tone,
  className,
}: {
  tone: StateTone;
  className?: string;
}) {
  const Icon = TONE_ICON[tone];
  const toneText: Record<StateTone, string> = {
    ok: "text-success",
    warning: "text-copy",
    error: "text-danger",
    info: "text-accent",
    neutral: "text-copy-muted",
  };
  return (
    <Icon
      aria-hidden="true"
      className={cn("size-4 shrink-0", toneText[tone], className)}
    />
  );
}
