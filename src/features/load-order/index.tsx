// Created by The Angelo Umali.
import { RepeatIcon } from "lucide-react";
import { useAppStore } from "@/components/app/app-store";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState, Panel } from "@/components/app/panel";
import { StateBadge } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { useLoadOrder, useMods } from "@/data/source";
import type { LoadOrderEntry, Mod } from "@/data/types";

function whyText(entry: LoadOrderEntry, nameOf: (modId: string) => string) {
  if (entry.reason === "late")
    return "Late phase, because it asks to load after everything";
  if (entry.reason === "load-after")
    return `After ${entry.after.map(nameOf).join(", ")}`;
  return "No load rules, so alphabetical by ModID";
}

function ruleText(entry: LoadOrderEntry) {
  if (entry.reason === "late") return 'LoadAfter: ["*"]';
  if (entry.reason === "load-after")
    return `LoadAfter: [${entry.after.map((modId) => `"${modId}"`).join(", ")}]`;
  return null;
}

export function LoadOrderScreen() {
  const { gameId, shortName } = useCurrentGame();
  const { data: order } = useLoadOrder(gameId);
  const { data: mods = [] } = useMods(gameId);
  const selectMod = useAppStore((store) => store.selectMod);
  const setView = useAppStore((store) => store.setView);
  const modById = new Map<string, Mod>(mods.map((mod) => [mod.id, mod]));
  const nameOf = (modId: string) => modById.get(modId)?.name ?? modId;
  const cycleMembers = new Set(order?.cycles.flat() ?? []);
  const entries = order?.entries ?? [];

  const openInLibrary = (modId: string) => {
    selectMod(modId);
    setView("library");
  };

  const phases = [
    {
      phase: "normal",
      title: "Normal phase",
      note: "Loads first, in dependency order",
    },
    { phase: "late", title: "Late phase", note: "Loads after everything else" },
  ] as const;

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Load order"
        description={`${shortName} decides the order. This view predicts it from each mod's rules and shows why every mod sits where it does.`}
      />
      {order?.cycles.map((cycle) => (
        <Alert
          key={cycle.join(">")}
          className="rounded-field border-danger/60 bg-bg-elevated text-copy"
        >
          <RepeatIcon aria-hidden="true" className="text-danger" />
          <AlertTitle className="font-semibold">LoadAfter loop</AlertTitle>
          <AlertDescription className="text-copy">
            <p>
              {cycle.map(nameOf).join(" → ")}. The game's order for these mods
              is undefined, so it falls back to alphabetical. Remove one rule,
              or turn one of them off.
            </p>
            <button
              type="button"
              className="mt-1 text-accent underline-offset-4 hover:underline"
              onClick={() => openInLibrary(cycle[0] ?? "")}
            >
              Open {nameOf(cycle[0] ?? "")} in Library
            </button>
          </AlertDescription>
        </Alert>
      ))}
      {order && entries.length === 0 ? (
        <EmptyState icon={<RepeatIcon />} title="Nothing to order">
          Turn mods on in the Library and their predicted order appears here.
        </EmptyState>
      ) : (
        phases.map(({ phase, title, note }) => {
          const phaseEntries = entries.filter((entry) => entry.phase === phase);
          return (
            <Panel
              key={phase}
              title={title}
              actions={
                <span className="text-label text-copy-dim">
                  {phaseEntries.length} mods · {note}
                </span>
              }
            >
              {phaseEntries.length === 0 ? (
                <p className="text-ui text-copy-muted">
                  No mods in this phase.
                </p>
              ) : (
                <ol className="flex flex-col divide-y divide-line">
                  {phaseEntries.map((entry) => {
                    const mod = modById.get(entry.modId);
                    const rule = ruleText(entry);
                    return (
                      <li
                        key={entry.modId}
                        className="flex items-start gap-3 py-2.5"
                      >
                        <span className="w-7 shrink-0 pt-0.5 font-mono text-label text-copy-dim tabular-nums">
                          {String(entry.position).padStart(2, "0")}
                        </span>
                        <div className="min-w-0 flex-1">
                          <button
                            type="button"
                            onClick={() => openInLibrary(entry.modId)}
                            className="max-w-full truncate text-left text-ui font-semibold hover:text-accent"
                          >
                            <span className="font-normal text-copy-muted">
                              [{mod?.category}]{" "}
                            </span>
                            {nameOf(entry.modId)}
                          </button>
                          <p className="text-ui text-copy-muted">
                            {whyText(entry, nameOf)}
                            {rule && (
                              <span className="ml-2 font-mono text-label text-copy-dim">
                                {rule}
                              </span>
                            )}
                          </p>
                        </div>
                        {cycleMembers.has(entry.modId) && (
                          <StateBadge tone="warning" icon={RepeatIcon}>
                            In a loop
                          </StateBadge>
                        )}
                      </li>
                    );
                  })}
                </ol>
              )}
            </Panel>
          );
        })
      )}
    </div>
  );
}
