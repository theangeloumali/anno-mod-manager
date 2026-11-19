// Created by The Angelo Umali.
import {
  CheckIcon,
  DownloadIcon,
  FlagIcon,
  ThumbsDownIcon,
  ThumbsUpIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { formatBytes, formatCount } from "@/components/app/format";
import { ModBanner, PoweredByModio } from "@/components/app/mod-badges";
import { withToast } from "@/components/app/notify";
import { StateBadge } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useAccount, useAction } from "@/data/source";
import type { ModioMod } from "@/data/types";

type Rating = "up" | "down" | null;

export function ModioDetailSheet({
  mod,
  onClose,
}: {
  mod: ModioMod | null;
  onClose: () => void;
}) {
  const { gameId, isWriteLocked, shortName } = useCurrentGame();
  const { data: account } = useAccount();
  const subscribe = useAction((source, modioId: number) =>
    source.subscribe(gameId, modioId),
  );
  // The contract has no rate or report call yet (report DEVIATIONS); these stay local in the mock-up.
  const [rating, setRating] = useState<Rating>(null);
  const isSignedIn = account?.signedIn ?? false;

  const rate = (nextRating: Exclude<Rating, null>) => {
    setRating((current) => (current === nextRating ? null : nextRating));
    toast.success("Rating sent to mod.io");
  };

  return (
    <Sheet
      open={mod !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          setRating(null);
          onClose();
        }
      }}
    >
      <SheetContent className="w-full gap-0 bg-bg-elevated p-0 data-[side=right]:sm:max-w-lg">
        {mod && (
          <>
            <ModBanner name={mod.name} className="h-36 w-full text-h2" />
            <SheetHeader className="pr-14">
              <SheetTitle className="text-panel-title font-bold">
                {mod.name}
              </SheetTitle>
              <SheetDescription className="text-ui text-copy-muted">
                by {mod.author} ·{" "}
                <span className="font-mono text-label">{mod.nameId}</span>
              </SheetDescription>
            </SheetHeader>
            <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pb-4">
              <div className="flex flex-wrap gap-1.5">
                {mod.installedModId && (
                  <StateBadge tone="ok">Installed</StateBadge>
                )}
                {mod.subscribed && (
                  <StateBadge tone="info" icon={CheckIcon}>
                    Subscribed
                  </StateBadge>
                )}
                {mod.tags.map((tag) => (
                  <StateBadge key={tag} tone="neutral">
                    {tag}
                  </StateBadge>
                ))}
              </div>
              <p className="text-ui text-copy">{mod.summary}</p>
              <dl className="grid grid-cols-3 gap-3 rounded-field border border-line bg-bg p-3">
                {[
                  ["Downloads", formatCount(mod.downloads)],
                  ["Rating", `${mod.rating}%`],
                  ["Size", formatBytes(mod.sizeBytes)],
                ].map(([term, value]) => (
                  <div key={term}>
                    <dt className="text-label text-copy-dim">{term}</dt>
                    <dd className="font-mono text-ui tabular-nums">{value}</dd>
                  </div>
                ))}
              </dl>
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-label text-copy-muted">Rate it</span>
                <Button
                  size="sm"
                  aria-pressed={rating === "up"}
                  disabled={!isSignedIn}
                  onClick={() => rate("up")}
                >
                  <ThumbsUpIcon aria-hidden="true" />
                  Good
                </Button>
                <Button
                  size="sm"
                  aria-pressed={rating === "down"}
                  disabled={!isSignedIn}
                  onClick={() => rate("down")}
                >
                  <ThumbsDownIcon aria-hidden="true" />
                  Not for me
                </Button>
                <Button
                  size="sm"
                  variant="quiet"
                  onClick={() =>
                    toast.info("mod.io's report form opens in your browser.")
                  }
                >
                  <FlagIcon aria-hidden="true" />
                  Report
                </Button>
              </div>
              {!isSignedIn && (
                <p className="text-label text-copy-muted">
                  Sign in under Settings to subscribe and rate. Installing works
                  as a guest.
                </p>
              )}
            </div>
            <SheetFooter className="flex-row flex-wrap items-center justify-end border-t border-line">
              <span className="mr-auto">
                <PoweredByModio />
              </span>
              {mod.installedModId ? (
                <Button disabled>
                  <CheckIcon aria-hidden="true" />
                  Installed
                </Button>
              ) : (
                <Button
                  variant="primary"
                  disabled={isWriteLocked || subscribe.isPending}
                  onClick={() =>
                    withToast(
                      subscribe.mutateAsync([mod.id]),
                      `Downloading ${mod.name} for ${shortName}`,
                    )
                  }
                >
                  <DownloadIcon aria-hidden="true" />
                  {isSignedIn ? "Subscribe and install" : "Install"}
                </Button>
              )}
            </SheetFooter>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
