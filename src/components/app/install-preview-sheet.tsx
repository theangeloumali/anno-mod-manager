// Created by The Angelo Umali.
import { DownloadIcon, LockIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useAction } from "@/data/source";
import type { InstallItemStatus, InstallPreviewItem } from "@/data/types";
import { useAppStore } from "./app-store";
import { errorMessage, gameShortName } from "./format";
import { withToast } from "./notify";
import { StateBadge, StateIcon, type StateTone } from "./state-badge";
import { useCurrentGame } from "./use-current-game";

const STATUS_BADGE: Record<
  InstallItemStatus,
  { tone: StateTone; label: string }
> = {
  new: { tone: "ok", label: "New" },
  update: { tone: "info", label: "Update" },
  downgrade: { tone: "warning", label: "Downgrade" },
  same: { tone: "neutral", label: "Already installed" },
  "wrong-game": { tone: "error", label: "Wrong game" },
  invalid: { tone: "error", label: "Invalid" },
};

const CHECK_TONE = { ok: "ok", warning: "warning", error: "error" } as const;

/** Drop, file picker and the mock menu all open the preview through this. */
export function useStartInstallPreview() {
  const showPreview = useAppStore((store) => store.showPreview);
  const game = useAppStore((store) => store.game);
  const preview = useAction((source, paths: string[]) =>
    source.previewInstall(game, paths),
  );
  return async (paths: string[]) => {
    try {
      showPreview(await preview.mutateAsync([paths]));
    } catch (error) {
      toast.error(errorMessage(error));
    }
  };
}

function PreviewItem({
  item,
  includeDowngrades,
  onIncludeDowngradesChange,
}: {
  item: InstallPreviewItem;
  includeDowngrades: boolean;
  onIncludeDowngradesChange: (include: boolean) => void;
}) {
  const badge = STATUS_BADGE[item.status];
  const versionText = item.previousVersion
    ? `${item.previousVersion} → ${item.version}`
    : item.version;
  return (
    <li className="rounded-field border border-line bg-bg p-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-ui font-semibold">
            <span className="font-normal text-copy-muted">
              [{item.category}]{" "}
            </span>
            {item.name}
          </p>
          <p className="truncate font-mono text-label text-copy-dim">
            {item.modId} · {versionText}
          </p>
        </div>
        <StateBadge tone={badge.tone}>{badge.label}</StateBadge>
      </div>
      <ul className="mt-3 flex flex-col gap-1.5">
        {item.checks.map((check) => (
          <li key={check.message} className="flex gap-2 text-ui text-copy">
            <StateIcon tone={CHECK_TONE[check.level]} className="mt-1" />
            <span>{check.message}</span>
          </li>
        ))}
      </ul>
      {item.status === "downgrade" && (
        <label className="mt-3 flex items-center gap-2 text-ui">
          <Checkbox
            className="data-checked:border-accent data-checked:bg-accent data-checked:text-on-accent"
            checked={includeDowngrades}
            onCheckedChange={(checked) => onIncludeDowngradesChange(checked)}
          />
          Install the older version anyway
        </label>
      )}
    </li>
  );
}

export function InstallPreviewSheet() {
  const preview = useAppStore((store) => store.preview);
  const showPreview = useAppStore((store) => store.showPreview);
  const setGame = useAppStore((store) => store.setGame);
  const { gameId, isWriteLocked, lockReason } = useCurrentGame();
  const [includeDowngrades, setIncludeDowngrades] = useState(false);
  const confirm = useAction(
    (source, previewId: string, withDowngrades: boolean) =>
      source.confirmInstall(previewId, withDowngrades),
  );
  const cancel = useAction((source, previewId: string) =>
    source.cancelInstall(previewId),
  );

  const items = preview?.items ?? [];
  const subModCount = items.reduce(
    (sum, item) => sum + item.subModIds.length,
    0,
  );
  const installableCount = items.filter(
    (item) =>
      item.status === "new" ||
      item.status === "update" ||
      (item.status === "downgrade" && includeDowngrades),
  ).length;
  const wrongGameTarget =
    items.length > 0 && items.every((item) => item.status === "wrong-game")
      ? items[0]?.targetGame
      : null;

  const close = (shouldCancel: boolean) => {
    if (preview && shouldCancel) void cancel.mutateAsync([preview.id]);
    setIncludeDowngrades(false);
    showPreview(null);
  };

  const install = async () => {
    if (!preview) return;
    const result = await withToast(
      confirm.mutateAsync([preview.id, includeDowngrades]),
      (installResult) =>
        `Installed ${installResult.installed.length} mod${installResult.installed.length === 1 ? "" : "s"}` +
        (installResult.skipped.length
          ? `; skipped ${installResult.skipped.length}`
          : ""),
    );
    if (!result) return;
    if (wrongGameTarget) setGame(wrongGameTarget);
    close(false);
  };

  const title = `Found ${items.length} mod${items.length === 1 ? "" : "s"}${
    subModCount
      ? ` and ${subModCount} sub-mod${subModCount === 1 ? "" : "s"}`
      : ""
  } in ${preview?.sourceName ?? ""}`;

  return (
    <Sheet
      open={preview !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) close(true);
      }}
    >
      <SheetContent className="w-full gap-0 bg-bg-elevated p-0 data-[side=right]:sm:max-w-xl">
        <SheetHeader className="border-b border-line pr-14">
          <SheetTitle className="text-panel-title font-semibold">
            {title}
          </SheetTitle>
          <SheetDescription className="text-ui text-copy-muted">
            Install preview for {gameShortName(gameId)}. Nothing has been
            written yet.
          </SheetDescription>
        </SheetHeader>
        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          {isWriteLocked && (
            <Alert className="rounded-field border-line-strong bg-surface text-copy">
              <LockIcon aria-hidden="true" />
              <AlertDescription className="text-copy">
                {lockReason} This preview stays open.
              </AlertDescription>
            </Alert>
          )}
          <ul className="flex flex-col gap-3">
            {items.map((item) => (
              <PreviewItem
                key={item.modId}
                item={item}
                includeDowngrades={includeDowngrades}
                onIncludeDowngradesChange={setIncludeDowngrades}
              />
            ))}
          </ul>
        </div>
        <SheetFooter className="flex-row flex-wrap items-center justify-end border-t border-line">
          <span className="mr-auto text-label text-copy-dim">
            Nothing has been written yet.
          </span>
          <Button variant="quiet" onClick={() => close(true)}>
            Cancel
          </Button>
          {wrongGameTarget ? (
            <Button
              variant="primary"
              disabled={confirm.isPending}
              onClick={() => void install()}
            >
              <DownloadIcon aria-hidden="true" />
              Install for {gameShortName(wrongGameTarget)} instead
            </Button>
          ) : (
            <Button
              variant="primary"
              disabled={
                installableCount === 0 || isWriteLocked || confirm.isPending
              }
              onClick={() => void install()}
            >
              <DownloadIcon aria-hidden="true" />
              Install {installableCount} mod{installableCount === 1 ? "" : "s"}
            </Button>
          )}
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
