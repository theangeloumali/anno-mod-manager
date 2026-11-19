// Created by The Angelo Umali.
import { useGames } from "@/data/source";
import { useAppStore } from "./app-store";
import { gameExe, gameShortName } from "./format";

/** The selected game plus its write lock: while the game runs, nothing in its mods folder may change. */
export function useCurrentGame() {
  const gameId = useAppStore((store) => store.game);
  const { data: games } = useGames();
  const game = games?.find((candidate) => candidate.id === gameId);
  const isWriteLocked = game?.running ?? false;
  return {
    gameId,
    game,
    shortName: gameShortName(gameId),
    isWriteLocked,
    lockReason: isWriteLocked
      ? `${gameExe(gameId)} is running. Close the game to change mods.`
      : null,
  };
}
