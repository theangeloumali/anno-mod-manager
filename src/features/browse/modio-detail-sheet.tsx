// Created by The Angelo Umali.
import {
  CheckIcon,
  FlagIcon,
  ThumbsDownIcon,
  ThumbsUpIcon,
} from "lucide-react";
import { useState } from "react";
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
import { useAccount, useAction, useModioDetail } from "@/data/source";
import type { ModioMod } from "@/data/types";
import { ReportForm } from "./report-form";
import { SubscribeButton } from "./subscribe-button";

type Rating = "up" | "down" | null;

export function ModioDetailSheet({
  mod,
  onClose,
}: {
  mod: ModioMod | null;
  onClose: () => void;
}) {
  const { gameId } = useCurrentGame();
  const { data: account } = useAccount();
  const sendRating = useAction((source, modioId: number, up: boolean) =>
    source.rateMod(gameId, modioId, up),
  );
  const { data: detail } = useModioDetail(gameId, mod?.id ?? null);
  // mod.io keeps no per-user rating we can read back, so this only marks the last vote sent.
  const [rating, setRating] = useState<Rating>(null);
  const [isReporting, setReporting] = useState(false);
  const isSignedIn = account?.signedIn ?? false;

  const rate = async (modioId: number, nextRating: Exclude<Rating, null>) => {
    const sent = await withToast(
      sendRating.mutateAsync([modioId, nextRating === "up"]),
      "Rating sent to mod.io",
    );
    if (sent.ok) setRating(nextRating);
  };

  return (
    <Sheet
      open={mod !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          setRating(null);
          setReporting(false);
          onClose();
        }
      }}
    >
      <SheetContent className="w-full gap-0 bg-bg-elevated p-0 data-[side=right]:sm:max-w-lg">
        {mod && (
          <>
            <ModBanner
              name={mod.name}
              logoUrl={mod.logoUrl}
              className="h-36 w-full text-h2"
            />
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
              <p className="whitespace-pre-line text-ui text-copy">
                {detail?.description || mod.summary}
              </p>
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
                  disabled={!isSignedIn || sendRating.isPending}
                  onClick={() => void rate(mod.id, "up")}
                >
                  <ThumbsUpIcon aria-hidden="true" />
                  Good
                </Button>
                <Button
                  size="sm"
                  aria-pressed={rating === "down"}
                  disabled={!isSignedIn || sendRating.isPending}
                  onClick={() => void rate(mod.id, "down")}
                >
                  <ThumbsDownIcon aria-hidden="true" />
                  Not for me
                </Button>
                <Button
                  size="sm"
                  variant="quiet"
                  aria-expanded={isReporting}
                  onClick={() => setReporting((open) => !open)}
                >
                  <FlagIcon aria-hidden="true" />
                  Report
                </Button>
              </div>
              {isReporting && (
                <ReportForm
                  modioId={mod.id}
                  onDone={() => setReporting(false)}
                />
              )}
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
              <SubscribeButton mod={mod} />
            </SheetFooter>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
