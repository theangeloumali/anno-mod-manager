// Created by The Angelo Umali.
import { CircleCheckIcon } from "lucide-react";
import { useState } from "react";
import { SWITCH_CLASS } from "@/components/app/field-class";
import { IssueCard } from "@/components/app/issue-card";
import { PageHeader } from "@/components/app/page-header";
import { Panel } from "@/components/app/panel";
import {
  StateBadge,
  StateIcon,
  type StateTone,
} from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Switch } from "@/components/ui/switch";
import {
  useHealthChecks,
  useIssues,
  useLoaderLog,
  useMods,
} from "@/data/source";
import type { LogEntry } from "@/data/types";

const LOG_TONE: Record<LogEntry["level"], StateTone> = {
  info: "neutral",
  warning: "warning",
  error: "error",
};

const rawLine = (entry: LogEntry) =>
  `[${entry.time}] [${entry.level}] ${entry.modId ? `${entry.modId}: ` : ""}${entry.message}${entry.source ? ` (${entry.source})` : ""}`;

export function HealthScreen() {
  const { gameId, game, isWriteLocked, shortName } = useCurrentGame();
  const { data: issues = [] } = useIssues(gameId);
  const { data: checks = [] } = useHealthChecks(gameId);
  const { data: log = [] } = useLoaderLog(gameId);
  const { data: mods = [] } = useMods(gameId);
  const [isShowingInfo, setShowingInfo] = useState(false);
  const errorCount = issues.filter((issue) => issue.level === "error").length;
  const logCards = log.filter(
    (entry) => isShowingInfo || entry.level !== "info",
  );

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Health"
        description={`Problems Atelier can predict before ${shortName} starts, plus what the game's mod loader reported last time it ran.`}
        actions={
          errorCount ? (
            <StateBadge tone="error">{errorCount} errors</StateBadge>
          ) : (
            <StateBadge tone="ok">Ready to launch</StateBadge>
          )
        }
      />
      <div className="grid items-start gap-5 lg:grid-cols-2">
        <Panel title={`Issues (${issues.length})`}>
          {issues.length === 0 ? (
            <p className="flex items-center gap-2 text-ui text-copy-muted">
              <CircleCheckIcon
                aria-hidden="true"
                className="size-4 text-success"
              />
              No problems found in the active set.
            </p>
          ) : (
            <div className="flex flex-col gap-2">
              {issues.map((issue) => (
                <IssueCard
                  key={`${issue.kind}-${issue.modId}-${issue.relatedModId}`}
                  issue={issue}
                  mods={mods}
                  gameId={gameId}
                  isWriteLocked={isWriteLocked}
                />
              ))}
            </div>
          )}
        </Panel>
        <Panel title="Environment">
          <ul className="flex flex-col divide-y divide-line">
            {checks.map((check) => (
              <li key={check.id} className="flex gap-3 py-2.5">
                <StateIcon tone={check.level} className="mt-1" />
                <div className="min-w-0">
                  <p className="text-ui font-semibold">{check.title}</p>
                  <p className="text-label break-words text-copy-muted">
                    {check.detail}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        </Panel>
      </div>
      <Panel
        title="Mod loader log"
        actions={
          <label className="flex items-center gap-2 text-label text-copy-muted">
            <Switch
              className={SWITCH_CLASS}
              checked={isShowingInfo}
              onCheckedChange={setShowingInfo}
            />
            Show info lines
          </label>
        }
      >
        <ul className="flex flex-col gap-2">
          {logCards.map((entry, index) => (
            <li
              key={`${entry.time}-${index}`}
              className="rounded-field border border-line bg-bg p-3"
            >
              <div className="flex flex-wrap items-center gap-2">
                <StateBadge tone={LOG_TONE[entry.level]}>
                  {entry.level}
                </StateBadge>
                <span className="font-mono text-label text-copy-dim">
                  {entry.time}
                </span>
                {entry.modId && (
                  <span className="font-mono text-label text-accent">
                    {entry.modId}
                  </span>
                )}
              </div>
              <p className="mt-1.5 text-ui break-words">{entry.message}</p>
              {entry.source && (
                <p className="font-mono text-label text-copy-dim">
                  {entry.source}
                </p>
              )}
            </li>
          ))}
        </ul>
      </Panel>
      <Panel
        title="Raw log"
        actions={
          <span className="max-w-full truncate font-mono text-label text-copy-dim">
            {game?.logPath}
          </span>
        }
      >
        <pre
          tabIndex={0}
          aria-label="Raw mod-loader.log"
          className="max-h-72 overflow-auto rounded-field border border-line bg-bg p-3 font-mono text-label leading-relaxed whitespace-pre text-copy-muted"
        >
          {log.map(rawLine).join("\n")}
        </pre>
      </Panel>
    </div>
  );
}
