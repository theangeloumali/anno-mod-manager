// Created by The Angelo Umali.
import {
  CloudIcon,
  CloudOffIcon,
  FolderLockIcon,
  FolderOpenIcon,
  UserIcon,
} from "lucide-react";
import { useAccount, useConnectivity, useMods } from "@/data/source";
import { useCurrentGame } from "./use-current-game";

export function StatusBar() {
  const { gameId, game, isWriteLocked, shortName } = useCurrentGame();
  const { data: mods = [] } = useMods(gameId);
  const { data: account } = useAccount();
  const { data: isOnline } = useConnectivity();
  const isOffline = isOnline === false;
  const activeCount = mods.filter((mod) => mod.active).length;

  return (
    <footer className="flex flex-wrap items-center gap-x-5 gap-y-1 border-t border-line bg-bg-elevated px-4 py-1.5 text-label text-copy-muted">
      <span className="font-mono tabular-nums">
        {activeCount}/{mods.length} active
      </span>
      <span className="inline-flex items-center gap-1.5">
        {isOffline ? (
          <CloudOffIcon aria-hidden="true" className="size-3.5 text-danger" />
        ) : (
          <CloudIcon aria-hidden="true" className="size-3.5 text-success" />
        )}
        mod.io {isOffline ? "offline" : "online"}
      </span>
      <span className="inline-flex items-center gap-1.5">
        <UserIcon aria-hidden="true" className="size-3.5" />
        {account?.signedIn ? account.username : "Guest"}
      </span>
      <span className="inline-flex min-w-0 items-center gap-1.5">
        {isWriteLocked ? (
          <FolderLockIcon aria-hidden="true" className="size-3.5 text-danger" />
        ) : (
          <FolderOpenIcon aria-hidden="true" className="size-3.5" />
        )}
        {isWriteLocked
          ? `Locked while ${shortName} runs`
          : "Mods folder writable"}
      </span>
      {game && (
        <span className="ml-auto hidden truncate font-mono lg:inline">
          {game.modsPath}
        </span>
      )}
    </footer>
  );
}
