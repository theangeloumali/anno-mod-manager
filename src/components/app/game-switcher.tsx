// Created by The Angelo Umali.
import { GAME_IDS } from "@/data/types";
import { cn } from "@/lib/utils";
import { useAppStore } from "./app-store";
import { gameShortName } from "./format";

/** The switch changes only this badge and the copy; the palette stays ZKidz for both games. */
export function GameSwitcher() {
  const game = useAppStore((store) => store.game);
  const setGame = useAppStore((store) => store.setGame);
  return (
    <div
      role="group"
      aria-label="Game"
      className="inline-flex rounded-control border border-line-strong p-0.5"
    >
      {GAME_IDS.map((gameId) => {
        const isSelected = gameId === game;
        return (
          <button
            key={gameId}
            type="button"
            aria-pressed={isSelected}
            onClick={() => setGame(gameId)}
            className={cn(
              "min-h-9 rounded-control px-4 text-ui font-semibold transition-colors duration-(--dur-1)",
              isSelected
                ? "bg-accent text-on-accent"
                : "text-copy-muted hover:text-copy",
            )}
          >
            {gameShortName(gameId)}
          </button>
        );
      })}
    </div>
  );
}
