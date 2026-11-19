// Created by The Angelo Umali.
import { isTauri } from "@tauri-apps/api/core";
import { FolderOpenIcon, RotateCcwIcon } from "lucide-react";
import { useId, type ReactNode } from "react";
import { toast } from "sonner";
import { FIELD_CLASS } from "@/components/app/field-class";
import { errorMessage } from "@/components/app/format";
import { withToast } from "@/components/app/notify";
import { StateBadge } from "@/components/app/state-badge";
import { Button } from "@/components/ui/button";
import { pickFolder } from "@/data/ipc-source";
import { useAction } from "@/data/source";
import type { GameInfo, Store } from "@/data/types";

const STORE_LABEL: Record<Store, string> = {
  steam: "Steam",
  ubisoft: "Ubisoft Connect",
  epic: "Epic Games",
  manual: "Added by hand",
};

function PathField({
  label,
  value,
  children,
}: {
  label: string;
  value: string;
  children?: ReactNode;
}) {
  const inputId = useId();
  return (
    <div className="grid gap-1.5">
      <label
        htmlFor={inputId}
        className="text-label font-medium text-copy-muted"
      >
        {label}
      </label>
      <span className="flex gap-2">
        <input
          id={inputId}
          readOnly
          value={value}
          className={`${FIELD_CLASS} font-mono text-label`}
        />
        {children}
      </span>
    </div>
  );
}

/** Only the game folder is settable; the mods and log paths follow from it. */
function GameFolderActions({ game }: { game: GameInfo }) {
  const setPath = useAction((source, path: string | null) =>
    source.setGamePath(game.id, path),
  );
  const browse = async () => {
    if (!isTauri()) {
      toast.info("The folder picker opens in the desktop app.");
      return;
    }
    // withToast never throws, so only the picker needs its own catch.
    const folder = await pickFolder().catch((error: unknown) => {
      toast.error(errorMessage(error));
      return null;
    });
    if (folder)
      await withToast(setPath.mutateAsync([folder]), "Game folder saved");
  };
  return (
    <>
      <Button
        className="shrink-0"
        disabled={setPath.isPending}
        onClick={() => void browse()}
      >
        <FolderOpenIcon aria-hidden="true" />
        <span className="sr-only sm:not-sr-only">Browse</span>
      </Button>
      {game.store === "manual" && (
        <Button
          className="shrink-0"
          variant="quiet"
          disabled={setPath.isPending}
          onClick={() =>
            void withToast(
              setPath.mutateAsync([null]),
              "Back to the detected folder",
            )
          }
        >
          <RotateCcwIcon aria-hidden="true" />
          <span className="sr-only sm:not-sr-only">Auto-detect</span>
        </Button>
      )}
    </>
  );
}

export function GamePaths({ game }: { game: GameInfo }) {
  return (
    <div className="flex flex-col gap-3 rounded-field border border-line bg-bg p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-ui font-semibold">{game.displayName}</h3>
        {game.store ? (
          <StateBadge tone="ok">{STORE_LABEL[game.store]}</StateBadge>
        ) : (
          <StateBadge tone="warning">Not found</StateBadge>
        )}
      </div>
      <PathField label="Game folder" value={game.installPath ?? "Not detected"}>
        <GameFolderActions game={game} />
      </PathField>
      <PathField
        label="Mods folder (Atelier writes here)"
        value={game.modsPath}
      />
      <PathField label="Mod loader log" value={game.logPath} />
    </div>
  );
}
