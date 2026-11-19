// Created by The Angelo Umali.
import {
  CompassIcon,
  DownloadIcon,
  HeartPulseIcon,
  LayersIcon,
  ListOrderedIcon,
  PackageIcon,
  SettingsIcon,
  SlidersHorizontalIcon,
  type LucideIcon,
} from "lucide-react";
import { useDownloads, useIssues, useMods } from "@/data/source";
import { cn } from "@/lib/utils";
import { useAppStore, type ViewId } from "./app-store";

interface NavItem {
  view: ViewId;
  label: string;
  icon: LucideIcon;
  count?: number;
  countLabel?: string;
  isAlert?: boolean;
}

export function SidebarNav() {
  const game = useAppStore((store) => store.game);
  const view = useAppStore((store) => store.view);
  const setView = useAppStore((store) => store.setView);
  const { data: mods = [] } = useMods(game);
  const { data: issues = [] } = useIssues(game);
  const { data: jobs = [] } = useDownloads();

  const topLevelMods = mods.filter((mod) => mod.parentId === null);
  const activeJobs = jobs.filter(
    (job) => job.status !== "done" && job.status !== "failed",
  ).length;
  const failedJobs = jobs.filter((job) => job.status === "failed").length;
  const errorCount = issues.filter((issue) => issue.level === "error").length;

  const groups: { heading: string; items: NavItem[] }[] = [
    {
      heading: "Installed",
      items: [
        {
          view: "library",
          label: "Library",
          icon: LayersIcon,
          count: topLevelMods.length,
          countLabel: "mods",
        },
        { view: "load-order", label: "Load order", icon: ListOrderedIcon },
        {
          view: "tweaks",
          label: "Tweaks",
          icon: SlidersHorizontalIcon,
          count: mods.filter((mod) => mod.hasOptions).length,
          countLabel: "mods with options",
        },
      ],
    },
    {
      heading: "mod.io",
      items: [
        { view: "browse", label: "Browse", icon: CompassIcon },
        { view: "collections", label: "Collections", icon: PackageIcon },
        {
          view: "downloads",
          label: "Downloads",
          icon: DownloadIcon,
          count: failedJobs || activeJobs,
          countLabel: failedJobs ? "failed" : "in progress",
          isAlert: failedJobs > 0,
        },
      ],
    },
    {
      heading: "Check",
      items: [
        {
          view: "health",
          label: "Health",
          icon: HeartPulseIcon,
          count: issues.length,
          countLabel: "issues",
          isAlert: errorCount > 0,
        },
        { view: "settings", label: "Settings", icon: SettingsIcon },
      ],
    },
  ];

  return (
    <nav aria-label="Main" className="flex flex-col gap-5 p-3">
      {groups.map((group) => (
        <div key={group.heading}>
          <h2 className="px-3 pb-1 text-label font-semibold tracking-wide text-copy-dim uppercase">
            {group.heading}
          </h2>
          <ul className="flex flex-col gap-0.5">
            {group.items.map((item) => {
              const isCurrent = item.view === view;
              return (
                <li key={item.view}>
                  <button
                    type="button"
                    aria-current={isCurrent ? "page" : undefined}
                    onClick={() => setView(item.view)}
                    className={cn(
                      "flex min-h-10 w-full items-center gap-3 rounded-control px-3 text-left text-ui transition-colors duration-(--dur-hover)",
                      isCurrent
                        ? "relative bg-surface font-semibold text-copy before:absolute before:inset-y-2 before:left-0 before:w-[3px] before:rounded-pill before:bg-accent"
                        : "text-copy-muted hover:bg-surface hover:text-copy",
                    )}
                  >
                    <item.icon
                      aria-hidden="true"
                      className={cn("size-4", isCurrent && "text-accent")}
                    />
                    <span className="flex-1">{item.label}</span>
                    {item.count ? (
                      <span
                        className={cn(
                          "rounded-pill px-2 font-mono text-label tabular-nums",
                          item.isAlert
                            ? "bg-danger/15 text-danger"
                            : "text-copy-dim",
                        )}
                      >
                        {item.count}
                        <span className="sr-only"> {item.countLabel}</span>
                      </span>
                    ) : null}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}
