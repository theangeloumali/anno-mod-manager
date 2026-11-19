// Created by The Angelo Umali.
import { CloudOffIcon, SearchIcon, StarIcon } from "lucide-react";
import { useState } from "react";
import { ChoiceSelect } from "@/components/app/choice-select";
import { FIELD_CLASS, SWITCH_CLASS } from "@/components/app/field-class";
import { formatCount } from "@/components/app/format";
import { ModBanner, PoweredByModio } from "@/components/app/mod-badges";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState } from "@/components/app/panel";
import { StateBadge } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useAccount, useModioBrowse, useModioTags } from "@/data/source";
import type { ModioMod, ModioSort } from "@/data/types";
import { ModioDetailSheet } from "./modio-detail-sheet";

const PAGE_SIZE = 12;
const ALL_TAGS = "all";

const SORT_CHOICES: { value: ModioSort; label: string }[] = [
  { value: "trending", label: "Trending" },
  { value: "downloads", label: "Most downloaded" },
  { value: "rating", label: "Highest rated" },
  { value: "subscribers", label: "Most subscribers" },
  { value: "newest", label: "Newest" },
  { value: "updated", label: "Recently updated" },
];

function ModioCard({ mod, onOpen }: { mod: ModioMod; onOpen: () => void }) {
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        className="group flex h-full w-full flex-col overflow-hidden rounded-card border border-line bg-bg-elevated text-left transition-colors duration-(--dur-hover) hover:border-line-strong"
      >
        <ModBanner name={mod.name} className="h-24 w-full text-h3" />
        <div className="flex flex-1 flex-col gap-2 p-4">
          <div>
            <p className="text-ui font-semibold group-hover:text-accent">
              {mod.name}
            </p>
            <p className="text-label text-copy-muted">by {mod.author}</p>
          </div>
          <p className="line-clamp-2 text-label text-copy-muted">
            {mod.summary}
          </p>
          <div className="mt-auto flex flex-wrap items-center gap-x-3 gap-y-1.5 pt-1 text-label text-copy-dim">
            <span className="font-mono tabular-nums">
              {formatCount(mod.downloads)} downloads
            </span>
            <span className="inline-flex items-center gap-1 font-mono tabular-nums">
              <StarIcon aria-hidden="true" className="size-3" />
              {mod.rating}%
            </span>
            {mod.installedModId ? (
              <StateBadge tone="ok">Installed</StateBadge>
            ) : mod.subscribed ? (
              <StateBadge tone="info">Subscribed</StateBadge>
            ) : null}
          </div>
        </div>
      </button>
    </li>
  );
}

export function BrowseScreen() {
  const { gameId, game, shortName } = useCurrentGame();
  const { data: account } = useAccount();
  const [search, setSearch] = useState("");
  const [tag, setTag] = useState(ALL_TAGS);
  const [sort, setSort] = useState<ModioSort>("trending");
  const [isSubscribedOnly, setSubscribedOnly] = useState(false);
  const [offset, setOffset] = useState(0);
  const [openModId, setOpenModId] = useState<number | null>(null);
  const { data: tags = [] } = useModioTags(gameId);
  const {
    data: page,
    isPending,
    isError,
    refetch,
  } = useModioBrowse({
    game: gameId,
    search,
    tags: tag === ALL_TAGS ? [] : [tag],
    sort,
    subscribedOnly: isSubscribedOnly,
    offset,
  });
  const openMod = page?.items.find((item) => item.id === openModId) ?? null;
  const resetPaging =
    <TValue,>(setter: (value: TValue) => void) =>
    (value: TValue) => {
      setter(value);
      setOffset(0);
    };

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Browse mod.io"
        description={`Mods for ${game?.displayName ?? shortName}. Subscribing installs the mod and its required dependencies.`}
        actions={<PoweredByModio />}
      />
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-[minmax(0,1fr)_12rem_12rem_auto] lg:items-end">
        <label className="relative block sm:col-span-2 lg:col-span-1">
          <span className="sr-only">Search mod.io</span>
          <SearchIcon
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-copy-dim"
          />
          <input
            type="search"
            value={search}
            disabled={isError}
            onChange={(event) => resetPaging(setSearch)(event.target.value)}
            placeholder="Search mod.io"
            className={`${FIELD_CLASS} pl-9`}
          />
        </label>
        <ChoiceSelect
          label="Tag"
          value={tag}
          isDisabled={isError}
          choices={[
            { value: ALL_TAGS, label: "All tags" },
            ...tags.map((tagName) => ({ value: tagName, label: tagName })),
          ]}
          onChange={resetPaging(setTag)}
        />
        <ChoiceSelect
          label="Sort"
          value={sort}
          isDisabled={isError}
          choices={SORT_CHOICES}
          onChange={resetPaging(setSort)}
        />
        <label className="flex min-h-(--target-min) items-center gap-2 text-ui">
          <Switch
            className={SWITCH_CLASS}
            checked={isSubscribedOnly}
            disabled={!account?.signedIn || isError}
            onCheckedChange={resetPaging(setSubscribedOnly)}
          />
          Subscribed only
        </label>
      </div>

      {isError ? (
        <EmptyState
          icon={<CloudOffIcon />}
          title="mod.io is unreachable"
          actions={<Button onClick={() => void refetch()}>Try again</Button>}
        >
          You're offline or mod.io is down. Installed mods still work, and
          downloads you started resume when you reconnect.
        </EmptyState>
      ) : isPending ? (
        <ul
          className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4"
          aria-label="Loading mods"
        >
          {Array.from({ length: 8 }, (_, index) => (
            <li key={index}>
              <Skeleton className="h-60 rounded-card bg-surface" />
            </li>
          ))}
        </ul>
      ) : page && page.items.length === 0 ? (
        <EmptyState icon={<SearchIcon />} title="No mods match">
          Try a different search or clear the tag filter.
        </EmptyState>
      ) : (
        <ul className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
          {page?.items.map((mod) => (
            <ModioCard
              key={mod.id}
              mod={mod}
              onOpen={() => setOpenModId(mod.id)}
            />
          ))}
        </ul>
      )}

      {page && page.total > PAGE_SIZE && (
        <nav
          aria-label="Pages"
          className="flex items-center justify-center gap-3"
        >
          <Button
            size="sm"
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
          >
            Previous
          </Button>
          <span className="font-mono text-label text-copy-muted">
            {offset + 1}–{Math.min(offset + PAGE_SIZE, page.total)} of{" "}
            {page.total}
          </span>
          <Button
            size="sm"
            disabled={offset + PAGE_SIZE >= page.total}
            onClick={() => setOffset(offset + PAGE_SIZE)}
          >
            Next
          </Button>
        </nav>
      )}

      <ModioDetailSheet mod={openMod} onClose={() => setOpenModId(null)} />
    </div>
  );
}
