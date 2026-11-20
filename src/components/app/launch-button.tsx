// Created by The Angelo Umali.
import { LockIcon, PlayIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { hasLegacyLoader } from "@/data/health";
import { useAction, useHealthChecks, useIssues } from "@/data/source";
import { withToast } from "./notify";
import { useCurrentGame } from "./use-current-game";

/** The shell's gold action. Running = locked; errors warn in the label but never block launch. */
export function LaunchButton() {
  const { gameId, shortName, isWriteLocked } = useCurrentGame();
  const { data: issues = [] } = useIssues(gameId);
  const { data: checks = [] } = useHealthChecks(gameId);
  const launch = useAction((source) => source.launchGame(gameId));
  const errorCount = issues.filter((issue) => issue.level === "error").length;

  if (isWriteLocked)
    return (
      <Button variant="secondary" disabled focusableWhenDisabled>
        <LockIcon aria-hidden="true" />
        <span>
          {shortName} <span className="hidden sm:inline">is </span>running
        </span>
      </Button>
    );

  return (
    <Button
      variant="primary"
      disabled={launch.isPending}
      onClick={() =>
        withToast(
          launch.mutateAsync([]),
          hasLegacyLoader(checks)
            ? `Launching ${shortName} with the old mod loader still installed. Remove it under Health`
            : errorCount
              ? `Launching ${shortName} with ${errorCount} unresolved errors`
              : `Launching ${shortName}`,
        )
      }
    >
      <PlayIcon aria-hidden="true" />
      Launch<span className="hidden sm:inline"> {shortName}</span>
      {errorCount > 0 && (
        <span className="sr-only">, {errorCount} errors still need fixing</span>
      )}
    </Button>
  );
}
