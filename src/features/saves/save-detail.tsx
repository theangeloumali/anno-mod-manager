// Created by The Angelo Umali.
import { WandSparklesIcon } from "lucide-react";
import { useState } from "react";
import { withToast } from "@/components/app/notify";
import { Panel } from "@/components/app/panel";
import { StateBadge, type StateTone } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useAction, useSaveInspection } from "@/data/source";
import type { ModCheck, Save, SaveLoadReport } from "@/data/types";
import { cn } from "@/lib/utils";

export const saveFacts = (save: Save) =>
  [
    save.build && `Build ${save.build}`,
    save.worldSize,
    save.empireCount !== null && `${save.empireCount} empires`,
    save.mods.length === 1 ? "1 mod" : `${save.mods.length} mods`,
  ]
    .filter((fact) => typeof fact === "string" && fact !== "")
    .join(" · ");

function checkBadge(check: ModCheck): { tone: StateTone; text: string } {
  const status = check.status;
  switch (status.kind) {
    case "exact":
      return { tone: "ok", text: "Matches" };
    case "otherVersion":
      return {
        tone: "warning",
        text: `Installed ${status.installed}, save used ${check.version ?? "?"}`,
      };
    case "missing":
      return { tone: "error", text: "Not installed" };
    default: {
      const unhandled: never = status;
      return unhandled;
    }
  }
}

function ReportAlert({ report }: { report: SaveLoadReport }) {
  const isReady = report.registryWritten;
  return (
    <Alert
      className={cn(
        "rounded-field bg-surface text-copy",
        isReady ? "border-success/50" : "border-danger/60",
      )}
    >
      <AlertTitle className="font-semibold">
        {isReady
          ? "The mod list now matches this save"
          : "The mod list was left as it was"}
      </AlertTitle>
      <AlertDescription className="text-copy">
        {report.restored.length > 0 && (
          <p>
            Put back:{" "}
            {report.restored
              .map((mod) => `${mod.name} ${mod.version ?? ""}`.trim())
              .join(", ")}
            .
          </p>
        )}
        {!isReady && report.missing.length === 0 && (
          <p>Atelier couldn't write Registry.xml. Try again.</p>
        )}
        {!isReady && report.missing.length > 0 && (
          <p>
            Still not matching:{" "}
            {report.missing.map((check) => check.name).join(", ")}. Install{" "}
            {report.missing.length === 1 ? "it" : "them"} and try again;
            otherwise Humankind would reset the list.
          </p>
        )}
      </AlertDescription>
    </Alert>
  );
}

export function SaveDetail({ save }: { save: Save }) {
  const { gameId, isWriteLocked } = useCurrentGame();
  const { data: inspection, isError: isInspectionFailed } = useSaveInspection(
    gameId,
    save.path,
  );
  const [report, setReport] = useState<SaveLoadReport | null>(null);
  const makeLoadable = useAction((source, path: string) =>
    source.makeSaveLoadable(gameId, path),
  );
  const checks = inspection?.checks ?? [];

  const run = async () => {
    const outcome = await withToast(
      makeLoadable.mutateAsync([save.path]),
      (result) =>
        result.registryWritten
          ? "Mod list set for this save"
          : "Some mods still don't match",
    );
    if (outcome.ok) setReport(outcome.value);
  };

  return (
    <Panel title={<span className="wrap-anywhere">{save.name}</span>}>
      <p className="text-label text-copy-muted">{saveFacts(save)}</p>
      <h3 className="mt-4 mb-1 text-ui font-semibold text-copy">Mods</h3>
      {save.mods.length === 0 ? (
        <p className="text-ui text-copy-muted">
          Played without mods. It loads with only the base game on.
        </p>
      ) : !inspection ? (
        <p className="text-ui text-copy-muted">
          {isInspectionFailed
            ? "Atelier couldn't compare this save's mods with what's installed."
            : "Checking this save's mods…"}
        </p>
      ) : (
        <ul className="flex flex-col divide-y divide-line">
          {checks.map((check) => {
            const badge = checkBadge(check);
            return (
              <li
                key={check.name}
                className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 py-2"
              >
                <span className="min-w-0 text-ui wrap-anywhere text-copy">
                  {check.name}
                  {check.version && (
                    <span className="ml-2 font-mono text-label text-copy-dim">
                      {check.version}
                    </span>
                  )}
                </span>
                <StateBadge tone={badge.tone}>{badge.text}</StateBadge>
              </li>
            );
          })}
        </ul>
      )}
      <div className="mt-4 flex flex-col gap-3">
        <Button
          variant="primary"
          className="self-start"
          disabled={isWriteLocked || makeLoadable.isPending}
          onClick={() => void run()}
        >
          <WandSparklesIcon aria-hidden="true" data-icon="inline-start" />
          Make this save load
        </Button>
        <p className="text-label text-copy-muted">
          {isWriteLocked
            ? "Close Humankind first."
            : "Puts back backed-up versions, then sets the mod list only if every mod matches."}
        </p>
        <div aria-live="polite">
          {report && <ReportAlert report={report} />}
        </div>
      </div>
    </Panel>
  );
}
