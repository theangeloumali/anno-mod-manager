// Created by The Angelo Umali.
import {
  CloudOffIcon,
  DownloadIcon,
  HeartIcon,
  PackageIcon,
  UsersIcon,
} from "lucide-react";
import { useState } from "react";
import { formatCount } from "@/components/app/format";
import { ModBanner, PoweredByModio } from "@/components/app/mod-badges";
import { withToast } from "@/components/app/notify";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState } from "@/components/app/panel";
import { useCurrentGame } from "@/components/app/use-current-game";
import { WriteLockNotice } from "@/components/app/write-lock-notice";
import { Button } from "@/components/ui/button";
import { useAccount, useAction, useCollections } from "@/data/source";

export function CollectionsScreen() {
  const { gameId, isWriteLocked } = useCurrentGame();
  const { data: collections = [], isError, refetch } = useCollections(gameId);
  const { data: account } = useAccount();
  const install = useAction((source, collectionId: number) =>
    source.installCollection(gameId, collectionId),
  );
  // No follow call in the contract (report DEVIATIONS): the toggle is local in the mock-up.
  const [followOverrides, setFollowOverrides] = useState<
    Record<number, boolean>
  >({});

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Collections"
        description="Curated mod sets from mod.io. Installing one also creates a matching profile, so you can switch back to your own set in one click."
        actions={<PoweredByModio />}
      />
      <WriteLockNotice />
      {isError ? (
        <EmptyState
          icon={<CloudOffIcon />}
          title="mod.io is unreachable"
          actions={<Button onClick={() => void refetch()}>Try again</Button>}
        >
          Collections come from mod.io. Your installed mods and profiles still
          work offline.
        </EmptyState>
      ) : collections.length === 0 ? (
        <EmptyState icon={<PackageIcon />} title="No collections yet">
          Nobody has published a collection for this game yet.
        </EmptyState>
      ) : (
        <ul className="grid gap-4 md:grid-cols-2 2xl:grid-cols-3">
          {collections.map((collection) => {
            const isFollowing =
              followOverrides[collection.id] ?? collection.following;
            return (
              <li
                key={collection.id}
                className="flex flex-col overflow-hidden rounded-card border border-line bg-bg-elevated"
              >
                <ModBanner
                  name={collection.name}
                  className="h-20 w-full text-h3"
                />
                <div className="flex flex-1 flex-col gap-3 p-4">
                  <div>
                    <h2 className="text-ui-lg font-semibold">
                      {collection.name}
                    </h2>
                    <p className="text-label text-copy-muted">
                      by {collection.author}
                    </p>
                  </div>
                  <p className="text-ui text-copy-muted">
                    {collection.summary}
                  </p>
                  <p className="flex gap-4 font-mono text-label text-copy-dim">
                    <span className="inline-flex items-center gap-1">
                      <PackageIcon aria-hidden="true" className="size-3.5" />
                      {collection.modCount} mods
                    </span>
                    <span className="inline-flex items-center gap-1">
                      <UsersIcon aria-hidden="true" className="size-3.5" />
                      {formatCount(collection.followers)} followers
                    </span>
                  </p>
                  <div className="mt-auto flex flex-wrap gap-2 pt-1">
                    <Button
                      size="sm"
                      variant="steel"
                      disabled={isWriteLocked || install.isPending}
                      onClick={() =>
                        withToast(
                          install.mutateAsync([collection.id]),
                          `Installing ${collection.name}; profile “${collection.name}” created`,
                        )
                      }
                    >
                      <DownloadIcon aria-hidden="true" />
                      Install collection
                    </Button>
                    <Button
                      size="sm"
                      aria-pressed={isFollowing}
                      disabled={!account?.signedIn}
                      onClick={() =>
                        setFollowOverrides((current) => ({
                          ...current,
                          [collection.id]: !isFollowing,
                        }))
                      }
                    >
                      <HeartIcon
                        aria-hidden="true"
                        className={isFollowing ? "fill-current" : undefined}
                      />
                      {isFollowing ? "Following" : "Follow"}
                    </Button>
                  </div>
                  {!account?.signedIn && (
                    <p className="text-label text-copy-dim">
                      Sign in to follow collections.
                    </p>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
