// Created by The Angelo Umali.
import { HistoryIcon, MinusIcon, PlusIcon, UndoIcon } from "lucide-react";
import { useState } from "react";
import { withToast } from "@/components/app/notify";
import { StateBadge } from "@/components/app/state-badge";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useAction, useModListHistory } from "@/data/source";
import type { HistoryEntry } from "@/data/types";

const entryTime = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

/** ModListGuard writes the time; show it raw when it isn't a date. */
const formatTime = (time: string) =>
  Number.isNaN(Date.parse(time)) ? time : entryTime.format(new Date(time));

function EntryDiff({ entry }: { entry: HistoryEntry }) {
  const removed = entry.old.filter((module) => !entry.new.includes(module));
  const added = entry.new.filter((module) => !entry.old.includes(module));
  const rows = [
    ...removed.map((module) => ({ module, isAdded: false })),
    ...added.map((module) => ({ module, isAdded: true })),
  ];
  if (rows.length === 0)
    return <p className="text-ui text-copy-muted">Same mods, new order.</p>;
  return (
    <ul className="flex flex-col gap-1">
      {rows.map(({ module, isAdded }) => {
        const Icon = isAdded ? PlusIcon : MinusIcon;
        return (
          <li key={module} className="flex items-center gap-2 text-ui">
            <Icon
              aria-hidden="true"
              className={isAdded ? "size-4 text-success" : "size-4 text-danger"}
            />
            <span className="sr-only">{isAdded ? "Added" : "Removed"}:</span>
            <span className="min-w-0 wrap-anywhere text-copy">{module}</span>
          </li>
        );
      })}
    </ul>
  );
}

export function HistorySheet() {
  const [isOpen, setOpen] = useState(false);
  const { data: history = [] } = useModListHistory(isOpen);
  const restore = useAction(async (source, entry: HistoryEntry) => {
    // The plugin can log a change while the sheet is open, so re-find the entry's index.
    const current = await source.modListHistory();
    const index = current.findIndex(
      (candidate) =>
        candidate.time === entry.time &&
        candidate.old.join("\n") === entry.old.join("\n"),
    );
    if (index < 0)
      throw new Error(
        "That change is no longer in the history. Reopen History and try again.",
      );
    return source.restoreHistoryEntry(index);
  });

  return (
    <>
      <Button size="sm" onClick={() => setOpen(true)}>
        <HistoryIcon aria-hidden="true" />
        History
      </Button>
      <Sheet open={isOpen} onOpenChange={setOpen}>
        <SheetContent className="w-full gap-0 overflow-y-auto bg-bg-elevated p-0 data-[side=right]:sm:max-w-md">
          <SheetHeader className="border-b border-line p-5">
            <SheetTitle className="text-panel-title font-semibold">
              Mod list history
            </SheetTitle>
            <SheetDescription className="text-ui text-copy-muted">
              Every time the list in Registry.xml changed, and what it was
              before. Restore one to get it back as a profile.
            </SheetDescription>
          </SheetHeader>
          {history.length === 0 ? (
            <p className="p-5 text-ui text-copy-muted">
              No changes recorded yet. Atelier notes each change to the mod list
              from here on.
            </p>
          ) : (
            <ol className="flex flex-col divide-y divide-line">
              {history.map((entry, index) => (
                <li
                  key={`${entry.time}-${index}`}
                  className="flex flex-col gap-2 p-5"
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <time
                      dateTime={entry.time}
                      className="font-mono text-label text-copy-dim"
                    >
                      {formatTime(entry.time)}
                    </time>
                    <StateBadge tone={entry.culprit ? "error" : "neutral"}>
                      {entry.reason}
                    </StateBadge>
                  </div>
                  {entry.culprit && (
                    <p className="text-ui text-copy">
                      Caused by{" "}
                      <span className="font-semibold wrap-anywhere">
                        {entry.culprit}
                      </span>
                      , which isn't installed.
                    </p>
                  )}
                  <p className="text-label text-copy-muted">
                    {entry.old.length} modules before, {entry.new.length} after
                  </p>
                  <EntryDiff entry={entry} />
                  <Button
                    size="sm"
                    className="self-start"
                    disabled={restore.isPending}
                    onClick={() =>
                      withToast(
                        restore.mutateAsync([entry]),
                        (name) => `Saved the old list as profile “${name}”`,
                      )
                    }
                  >
                    <UndoIcon aria-hidden="true" />
                    Restore as profile
                  </Button>
                </li>
              ))}
            </ol>
          )}
        </SheetContent>
      </Sheet>
    </>
  );
}
