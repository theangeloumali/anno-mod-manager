import { CheckIcon, DownloadIcon, XIcon } from "lucide-react";
import { withToast } from "@/components/app/notify";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import { useAccount, useAction } from "@/data/source";
import type { ModioMod } from "@/data/types";

/**
 * Subscribe (signed in) or Install (guest); Unsubscribe once subscribed, which also removes
 * the installed copy into the backups. `compact` is the card's shorter label.
 */
export function SubscribeButton({
  mod,
  compact = false,
}: {
  mod: ModioMod;
  compact?: boolean;
}) {
  const { gameId, isWriteLocked, shortName } = useCurrentGame();
  const { data: account } = useAccount();
  const isSignedIn = account?.signedIn ?? false;
  const subscribe = useAction((source) => source.subscribe(gameId, mod.id));
  const unsubscribe = useAction((source) => source.unsubscribe(gameId, mod.id));
  const size = compact ? "sm" : "default";

  if (isSignedIn && mod.subscribed)
    return (
      <Button
        size={size}
        variant="quiet"
        disabled={isWriteLocked || unsubscribe.isPending}
        aria-label={`Unsubscribe from ${mod.name}`}
        onClick={() =>
          withToast(
            unsubscribe.mutateAsync([]),
            `Unsubscribed from ${mod.name}. A backup of it is kept.`,
          )
        }
      >
        <XIcon aria-hidden="true" />
        Unsubscribe
      </Button>
    );

  // Signed in, a mod installed as a guest can still be subscribed to, for updates and sync.
  if (mod.installedModId && !isSignedIn)
    return (
      <Button size={size} disabled>
        <CheckIcon aria-hidden="true" />
        Installed
      </Button>
    );

  const label = isSignedIn
    ? compact || mod.installedModId
      ? "Subscribe"
      : "Subscribe and install"
    : "Install";
  return (
    <Button
      size={size}
      variant="primary"
      disabled={isWriteLocked || subscribe.isPending}
      aria-label={`${label} ${mod.name}`}
      onClick={() =>
        withToast(
          subscribe.mutateAsync([]),
          mod.installedModId
            ? `Subscribed to ${mod.name}`
            : `Downloading ${mod.name} for ${shortName}`,
        )
      }
    >
      <DownloadIcon aria-hidden="true" />
      {label}
    </Button>
  );
}
