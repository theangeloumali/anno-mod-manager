// Created by The Angelo Umali.
import { CircleAlertIcon, LockIcon, PlayIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useAction, useIssues, useLaunchBlockers } from "@/data/source";
import { useAppStore } from "./app-store";
import { withToast } from "./notify";
import { useCurrentGame } from "./use-current-game";

/** The shell's gold action. Running = locked; errors warn in the label but never block launch. */
export function LaunchButton() {
  const { gameId, game, shortName, isWriteLocked } = useCurrentGame();
  const { data: issues = [] } = useIssues(gameId);
  const launch = useAction((source) => source.launchGame(gameId));
  const { data: blockers = [] } = useLaunchBlockers(
    gameId,
    gameId === "humankind",
  );
  const setView = useAppStore((state) => state.setView);
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

  // Humankind empties the mod list when a listed module is missing, so never launch into that.
  if (blockers.length > 0) {
    const missingList = blockers.join(", ");
    return (
      <Button
        variant="secondary"
        title={`Not installed: ${missingList}`}
        onClick={() => {
          setView("load-order");
          toast.error(
            `Not launching: ${missingList} ${blockers.length === 1 ? "is" : "are"} on the list but not installed, and ${shortName} would wipe the mod list. Fix it in Load order.`,
          );
        }}
      >
        <CircleAlertIcon aria-hidden="true" className="text-danger" />
        <span>
          {blockers.length} missing
          <span className="hidden sm:inline">
            {blockers.length === 1 ? " module" : " modules"}
          </span>
        </span>
        <span className="sr-only">
          . {shortName} can't launch until this is fixed in Load order.
        </span>
      </Button>
    );
  }

  return (
    <Button
      variant="primary"
      disabled={launch.isPending}
      onClick={() =>
        withToast(
          launch.mutateAsync([]),
          game?.legacyModLoader
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
