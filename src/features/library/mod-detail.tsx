// Created by The Angelo Umali.
import { RefreshCwIcon, SlidersHorizontalIcon, Trash2Icon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useAppStore } from "@/components/app/app-store";
import { formatBytes, plainDescription } from "@/components/app/format";
import { IssueCard } from "@/components/app/issue-card";
import {
  BreakableText,
  categoryPrefix,
  isReadOnly,
  LocationBadge,
  ModBanner,
} from "@/components/app/mod-badges";
import { withToast } from "@/components/app/notify";
import { Panel } from "@/components/app/panel";
import { StateBadge, type StateTone } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useAction } from "@/data/source";
import type { Issue, Mod, ModDependencies, ModLocation } from "@/data/types";
import { issuesForMod } from "./issues-for-mod";

const DEPENDENCY_GROUPS: { key: keyof ModDependencies; label: string }[] = [
  { key: "require", label: "Requires" },
  { key: "optional", label: "Works with" },
  { key: "loadAfter", label: "Loads after" },
  { key: "incompatible", label: "Incompatible with" },
  { key: "deprecate", label: "Replaces" },
];

function dependencyState(
  group: keyof ModDependencies,
  dependency: Mod | undefined,
): { tone: StateTone; label: string } {
  if (group === "incompatible")
    return dependency?.active
      ? { tone: "error", label: "On · conflicts" }
      : { tone: "ok", label: dependency ? "Off" : "Not installed" };
  if (!dependency)
    return group === "require"
      ? { tone: "error", label: "Missing" }
      : { tone: "neutral", label: "Not installed" };
  return dependency.active
    ? { tone: "ok", label: "On" }
    : { tone: "warning", label: "Off" };
}

function SaveSafety({ mod }: { mod: Mod }) {
  // Humankind has no modinfo; each save lists its own mods.
  if (mod.game === "humankind")
    return (
      <p className="text-ui text-copy-muted">
        Humankind mods don't say. The Saves page checks each save's mods.
      </p>
    );
  const { setup } = mod;
  const badges: { tone: StateTone; label: string }[] = [];
  if (setup.requiresNewGame)
    badges.push({ tone: "warning", label: "Needs a new game" });
  if (setup.safeToRemove === true)
    badges.push({ tone: "ok", label: "Safe to remove" });
  if (setup.safeToRemove === false)
    badges.push({ tone: "warning", label: "Not safe to remove mid-save" });
  if (setup.multiplayer === true)
    badges.push({ tone: "ok", label: "Multiplayer-safe" });
  if (setup.multiplayer === false)
    badges.push({ tone: "warning", label: "Single-player only" });
  if (setup.campaign) badges.push({ tone: "info", label: "Campaign-safe" });
  return (
    <div className="flex flex-wrap gap-1.5">
      {badges.length ? (
        badges.map((badge) => (
          <StateBadge key={badge.label} tone={badge.tone}>
            {badge.label}
          </StateBadge>
        ))
      ) : (
        <StateBadge tone="neutral">No save info in modinfo.json</StateBadge>
      )}
    </div>
  );
}

const GAME_FOLDER_NOTE =
  "Lives in the game folder. Atelier switches it on and off but never edits or removes it.";

/** Why a copy can't be removed here, by where it lives. */
const READ_ONLY_NOTE: Record<ModLocation, string> = {
  ubi: "Installed by the in-game mod browser. Ubisoft manages this folder, so Atelier only switches it on and off.",
  workshop:
    "From the Steam Workshop. Steam manages this folder, so Atelier only switches it on and off.",
  gamemodio:
    "Installed by Humankind's own mod.io browser. The game manages this folder, so Atelier only switches it on and off.",
  game: GAME_FOLDER_NOTE,
  documents: GAME_FOLDER_NOTE,
};

const STACKED_LAYOUT_QUERY = "(max-width: 79.99rem)";

const SECTION_HEADING =
  "mb-2 text-label font-semibold tracking-wide text-copy-dim uppercase";

export function ModDetail({
  mod,
  allMods,
  issues,
}: {
  mod: Mod | null;
  allMods: Mod[];
  issues: Issue[];
}) {
  const { gameId, isWriteLocked } = useCurrentGame();
  const openTweaks = useAppStore((store) => store.openTweaks);
  const selectMod = useAppStore((store) => store.selectMod);
  const [isConfirmOpen, setConfirmOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  // A copy is its folder: one ModID can sit in two locations.
  const selectedCopy = mod && `${mod.location}/${mod.folder}`;

  // A new selection opens at the card's top. Under xl the card stacks below the table, so it
  // would otherwise land off-screen.
  useEffect(() => {
    const panel = panelRef.current;
    if (!selectedCopy || !panel) return;
    panel.scrollTop = 0;
    if (window.matchMedia(STACKED_LAYOUT_QUERY).matches)
      panel.scrollIntoView({ block: "start" });
  }, [selectedCopy]);
  const uninstall = useAction((source, modId: string) =>
    source.uninstallMod(gameId, modId),
  );
  const restore = useAction((source, modId: string) =>
    source.restoreMod(gameId, modId),
  );
  const update = useAction((source, modId: string) =>
    source.updateMod(gameId, modId),
  );

  if (!mod)
    return (
      <Panel className="hidden xl:block">
        <p className="py-10 text-center text-ui text-copy-muted">
          Select a mod to see what it needs, whether it's safe for your save,
          and how to fix it.
        </p>
      </Panel>
    );

  const modIssues = issuesForMod(issues, mod.id);
  const findMod = (modId: string) =>
    allMods.find((candidate) => candidate.id === modId);
  const subMods = allMods.filter((candidate) => candidate.parentId === mod.id);
  const isLocked = isReadOnly(mod);

  return (
    // The wrapper sticks (the Panel alone spans its own box, so it never moved), capped to the scroll
    // area so a tall card scrolls inside it: 6rem of header and status bar, plus 1rem above and below.
    // Focusable so the keyboard can scroll it; the outline sits inside, where the scroller can't clip it.
    <div
      ref={panelRef}
      role="region"
      aria-label={`${mod.name} details`}
      tabIndex={0}
      className="scroll-mt-4 rounded-card outline-none focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring xl:sticky xl:top-4 xl:max-h-[calc(100dvh-8rem)] xl:overflow-y-auto"
    >
      <Panel className="flex flex-col gap-5" title={null}>
        <div className="flex gap-4">
          <ModBanner
            name={mod.name}
            className="size-16 shrink-0 rounded-field text-panel-title"
          />
          <div className="min-w-0">
            {/* Real names and ModIDs can be one long token, so these wrap anywhere, never clip. */}
            <h2 className="text-panel-title font-bold leading-tight wrap-anywhere">
              <BreakableText text={mod.name} />
            </h2>
            <p className="text-ui text-copy-muted wrap-anywhere">
              {[
                categoryPrefix(mod.category).trim(),
                mod.author && `by ${mod.author}`,
              ]
                .filter(Boolean)
                .join(" · ")}
            </p>
            <p className="font-mono text-label text-copy-dim wrap-anywhere">
              <BreakableText text={mod.id} />
            </p>
            <p className="font-mono text-label text-copy-dim wrap-anywhere">
              {mod.version} ·{" "}
              <span className="whitespace-nowrap">
                {formatBytes(mod.sizeBytes)}
              </span>
            </p>
          </div>
        </div>

        <div className="flex flex-wrap gap-1.5">
          <LocationBadge location={mod.location} />
          <StateBadge tone="neutral">
            {mod.origin === "modio" ? "From mod.io" : "Local file"}
          </StateBadge>
          {mod.difficulty && mod.difficulty !== "unchanged" && (
            <StateBadge
              tone={mod.difficulty === "cheat" ? "warning" : "neutral"}
            >
              Difficulty: {mod.difficulty}
            </StateBadge>
          )}
        </div>

        {isLocked && (
          <p className="rounded-field border border-line bg-bg p-3 text-ui text-copy-muted">
            {READ_ONLY_NOTE[mod.location]}
          </p>
        )}

        {mod.description && (
          <p className="whitespace-pre-line text-ui text-copy wrap-anywhere">
            {plainDescription(mod.description)}
          </p>
        )}

        <section>
          <h3 className={SECTION_HEADING}>Save safety</h3>
          <SaveSafety mod={mod} />
        </section>

        {modIssues.length > 0 && (
          <section className="flex flex-col gap-2">
            <h3 className={SECTION_HEADING}>Problems</h3>
            {modIssues.map((issue) => (
              <IssueCard
                key={`${issue.kind}-${issue.modId}-${issue.relatedModId}`}
                issue={issue}
                mods={allMods}
                gameId={gameId}
                isWriteLocked={isWriteLocked}
                showModName={issue.modId !== mod.id}
              />
            ))}
          </section>
        )}

        <section>
          <h3 className={SECTION_HEADING}>Dependencies</h3>
          <dl className="flex flex-col gap-3">
            {DEPENDENCY_GROUPS.filter(
              ({ key }) => mod.dependencies[key].length > 0,
            ).map(({ key, label }) => (
              <div key={key}>
                <dt className="text-label text-copy-muted">{label}</dt>
                {mod.dependencies[key].map((dependencyId) => {
                  if (dependencyId === "*")
                    return (
                      <dd key={dependencyId} className="text-ui">
                        Everything, so it loads in the late phase
                      </dd>
                    );
                  const dependency = findMod(dependencyId);
                  const state = dependencyState(key, dependency);
                  return (
                    <dd
                      key={dependencyId}
                      className="flex items-center justify-between gap-2 py-0.5"
                    >
                      <button
                        type="button"
                        disabled={!dependency}
                        onClick={() => selectMod(dependencyId)}
                        title={dependency?.name ?? dependencyId}
                        className="min-w-0 truncate text-left text-ui text-copy enabled:text-accent enabled:hover:underline"
                      >
                        {dependency?.name ?? (
                          <span className="font-mono text-label">
                            {dependencyId}
                          </span>
                        )}
                      </button>
                      <StateBadge tone={state.tone}>{state.label}</StateBadge>
                    </dd>
                  );
                })}
              </div>
            ))}
            {DEPENDENCY_GROUPS.every(
              ({ key }) => mod.dependencies[key].length === 0,
            ) && (
              <p className="text-ui text-copy-muted">
                No dependencies or load rules.
              </p>
            )}
          </dl>
        </section>

        {subMods.length > 0 && (
          <section>
            <h3 className={SECTION_HEADING}>Sub-mods</h3>
            <ul className="flex flex-col gap-1">
              {subMods.map((subMod) => (
                <li
                  key={subMod.id}
                  className="flex items-center justify-between gap-2 text-ui"
                >
                  <span className="min-w-0 truncate" title={subMod.name}>
                    {subMod.name}
                  </span>
                  <StateBadge tone={subMod.active ? "ok" : "neutral"}>
                    {subMod.active ? "On" : "Off"}
                  </StateBadge>
                </li>
              ))}
            </ul>
          </section>
        )}

        {mod.knownIssues.length > 0 && (
          <section>
            <h3 className={SECTION_HEADING}>Known issues (from the author)</h3>
            <ul className="list-disc pl-5 text-ui text-copy-muted">
              {mod.knownIssues.map((knownIssue) => (
                <li key={knownIssue}>{knownIssue}</li>
              ))}
            </ul>
          </section>
        )}

        <div className="flex flex-wrap gap-2 border-t border-line pt-4">
          {mod.hasOptions && (
            <Button size="sm" onClick={() => openTweaks(mod.id)}>
              <SlidersHorizontalIcon aria-hidden="true" />
              Tweaks
            </Button>
          )}
          {mod.update && mod.modioId !== null && !isLocked && (
            <Button
              size="sm"
              variant="steel"
              disabled={isWriteLocked || update.isPending}
              onClick={() =>
                withToast(
                  update.mutateAsync([mod.id]),
                  `Updating ${mod.name} to ${mod.update?.version}`,
                )
              }
            >
              <RefreshCwIcon aria-hidden="true" />
              Update to {mod.update.version}
            </Button>
          )}
          <Button
            size="sm"
            variant="quiet"
            disabled={isLocked || isWriteLocked}
            focusableWhenDisabled
            title={
              isLocked ? "Only mods in Documents can be removed" : undefined
            }
            onClick={() => setConfirmOpen(true)}
          >
            <Trash2Icon aria-hidden="true" />
            Uninstall
          </Button>
        </div>

        <Dialog open={isConfirmOpen} onOpenChange={setConfirmOpen}>
          <DialogContent className="bg-bg-elevated sm:max-w-md">
            <DialogHeader>
              <DialogTitle className="text-panel-title font-semibold">
                Uninstall {mod.name}?
              </DialogTitle>
              <DialogDescription className="text-ui text-copy-muted">
                Its folder moves to Atelier's backups, kept for as long as
                Settings says. Undo puts it straight back.
                {mod.setup.safeToRemove === false &&
                  " The author marks it as not safe to remove from an existing save."}
              </DialogDescription>
            </DialogHeader>
            <DialogFooter className="bg-transparent">
              <DialogClose render={<Button variant="quiet" />}>
                Keep it
              </DialogClose>
              <Button
                variant="steel"
                onClick={async () => {
                  const result = await withToast(
                    uninstall.mutateAsync([mod.id]),
                    `Uninstalled ${mod.name}`,
                    {
                      label: "Undo",
                      onClick: () =>
                        void withToast(
                          restore.mutateAsync([mod.id]),
                          `Restored ${mod.name}`,
                        ),
                    },
                  );
                  setConfirmOpen(false);
                  if (result.ok) selectMod(null);
                }}
              >
                Uninstall
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </Panel>
    </div>
  );
}
