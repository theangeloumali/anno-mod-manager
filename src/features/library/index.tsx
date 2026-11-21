// Created by The Angelo Umali.
import {
  CompassIcon,
  RefreshCwIcon,
  SearchIcon,
  UploadIcon,
} from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import { useRef, useState } from "react";
import { toast } from "sonner";
import { findSelectedMod, useAppStore } from "@/components/app/app-store";
import { FIELD_CLASS } from "@/components/app/field-class";
import { errorMessage } from "@/components/app/format";
import { useStartInstallPreview } from "@/components/app/install-preview-sheet";
import { withToast } from "@/components/app/notify";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState } from "@/components/app/panel";
import { useCurrentGame } from "@/components/app/use-current-game";
import { WriteLockNotice } from "@/components/app/write-lock-notice";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { pickArchives } from "@/data/ipc-source";
import { useAction, useIssues, useMods } from "@/data/source";
import type { Mod } from "@/data/types";
import { issuesForMod } from "./issues-for-mod";
import { ModDetail } from "./mod-detail";
import { ModTable } from "./mod-table";
import { ProfileBar } from "./profile-bar";

type LibraryFilter = "all" | "active" | "inactive" | "issues" | "updates";

export function LibraryScreen() {
  const { gameId, shortName, isWriteLocked } = useCurrentGame();
  const { data: mods = [], isPending } = useMods(gameId);
  const { data: issues = [] } = useIssues(gameId);
  const selectedModId = useAppStore((store) => store.selectedModId);
  const selectedCopy = useAppStore((store) => store.selectedModCopy);
  const setView = useAppStore((store) => store.setView);
  const startPreview = useStartInstallPreview();
  const updateAll = useAction((source) => source.updateAll(gameId));
  const checkUpdates = useAction((source) => source.checkUpdates(gameId));
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<LibraryFilter>("all");

  const topLevelMods = mods.filter((mod) => mod.parentId === null);
  const updatableCount = mods.filter(
    (mod) => mod.update && mod.location === "documents",
  ).length;
  const filterTests: Record<LibraryFilter, (mod: Mod) => boolean> = {
    all: () => true,
    active: (mod) => mod.active,
    inactive: (mod) => !mod.active,
    issues: (mod) => issuesForMod(issues, mod.id).length > 0,
    updates: (mod) => mod.update !== null,
  };

  const query = search.trim().toLowerCase();
  const matchesQuery = (mod: Mod) =>
    !query ||
    `${mod.name} ${mod.id} ${mod.author ?? ""} ${mod.category}`
      .toLowerCase()
      .includes(query);
  // A parent that matches shows all its sub-mods; otherwise only the sub-mods that match.
  const isSubModVisible = (subMod: Mod, parent: Mod) =>
    matchesQuery(parent) || matchesQuery(subMod);
  const visibleMods = topLevelMods.filter(
    (mod) =>
      filterTests[filter](mod) &&
      (matchesQuery(mod) ||
        mods.some(
          (subMod) => subMod.parentId === mod.id && matchesQuery(subMod),
        )),
  );

  const selectedMod = findSelectedMod(mods, selectedModId, selectedCopy);

  const fileButton = (
    <>
      <input
        ref={fileInputRef}
        type="file"
        multiple
        accept=".zip,.7z,.rar"
        className="sr-only"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(event) => {
          const fileNames = [...(event.target.files ?? [])].map(
            (file) => file.name,
          );
          if (fileNames.length) void startPreview(fileNames);
          event.target.value = "";
        }}
      />
      <Button
        onClick={() => {
          // The browser input only yields file names; the core needs full paths.
          if (!isTauri()) return fileInputRef.current?.click();
          pickArchives().then(
            (paths) => {
              if (paths.length) void startPreview(paths);
            },
            (error: unknown) => toast.error(errorMessage(error)),
          );
        }}
      >
        <UploadIcon aria-hidden="true" />
        Install from file
      </Button>
    </>
  );

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Library"
        description={`Every mod the ${shortName} loader can see, in Documents and in the game folder. Drop an archive anywhere on the window to install it.`}
        actions={
          <>
            {fileButton}
            <Button
              disabled={checkUpdates.isPending}
              onClick={() =>
                withToast(
                  checkUpdates.mutateAsync([]),
                  "Checked mod.io for updates",
                )
              }
            >
              <SearchIcon aria-hidden="true" />
              Check for updates
            </Button>
            {updatableCount > 0 && (
              <Button
                disabled={isWriteLocked || updateAll.isPending}
                onClick={() =>
                  withToast(
                    updateAll.mutateAsync([]),
                    `Updating ${updatableCount} mods`,
                  )
                }
              >
                <RefreshCwIcon aria-hidden="true" />
                Update all ({updatableCount})
              </Button>
            )}
          </>
        }
      />
      <WriteLockNotice />
      <ProfileBar />

      {!isPending && mods.length === 0 ? (
        <EmptyState
          icon={<CompassIcon />}
          title={`No mods for ${shortName} yet`}
          actions={
            <>
              <Button variant="steel" onClick={() => setView("browse")}>
                <CompassIcon aria-hidden="true" />
                Browse mod.io
              </Button>
              {fileButton}
            </>
          }
        >
          Drop a zip, 7z or rar on this window, or browse mod.io. Atelier checks
          every mod before anything is written.
        </EmptyState>
      ) : (
        <>
          <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <Tabs
              value={filter}
              onValueChange={(value) => setFilter(value as LibraryFilter)}
            >
              <TabsList className="w-full flex-wrap justify-start bg-surface group-data-horizontal/tabs:h-auto lg:w-fit">
                {(
                  [
                    ["all", "All"],
                    ["active", "On"],
                    ["inactive", "Off"],
                    ["issues", "Issues"],
                    ["updates", "Updates"],
                  ] as const
                ).map(([value, label]) => (
                  <TabsTrigger
                    key={value}
                    value={value}
                    className="h-auto min-h-8 flex-none px-3 text-ui"
                  >
                    {label}
                    <span className="font-mono text-label text-copy-dim tabular-nums">
                      {topLevelMods.filter(filterTests[value]).length}
                    </span>
                  </TabsTrigger>
                ))}
              </TabsList>
            </Tabs>
            <label className="relative block w-full lg:max-w-xs">
              <span className="sr-only">Search installed mods</span>
              <SearchIcon
                aria-hidden="true"
                className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-copy-dim"
              />
              <input
                type="search"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search by name, ModID or author"
                className={`${FIELD_CLASS} pl-9`}
              />
            </label>
          </div>
          <div className="grid grid-cols-[minmax(0,1fr)] items-start gap-5 xl:grid-cols-[minmax(0,1fr)_24rem]">
            <ModTable
              mods={visibleMods}
              allMods={mods}
              issues={issues}
              isSubModVisible={isSubModVisible}
            />
            <ModDetail mod={selectedMod} allMods={mods} issues={issues} />
          </div>
        </>
      )}
    </div>
  );
}
