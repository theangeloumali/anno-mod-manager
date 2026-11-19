// Created by The Angelo Umali.
import { LockIcon } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { useCurrentGame } from "./use-current-game";

/** Shown on every screen that writes, while the selected game is running. */
export function WriteLockNotice() {
  const { isWriteLocked, lockReason, shortName } = useCurrentGame();
  if (!isWriteLocked) return null;
  return (
    <Alert className="rounded-field border-line-strong bg-surface text-copy">
      <LockIcon aria-hidden="true" />
      <AlertTitle className="font-semibold">
        {shortName} is running, so mods are locked
      </AlertTitle>
      <AlertDescription className="text-copy-muted">
        {lockReason} You can still look around and plan changes.
      </AlertDescription>
    </Alert>
  );
}
