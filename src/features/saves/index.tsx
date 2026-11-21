// Created by The Angelo Umali.
import { FileWarningIcon, SaveIcon } from "lucide-react";
import { useState } from "react";
import { BreakableText } from "@/components/app/mod-badges";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState, Panel } from "@/components/app/panel";
import { useCurrentGame } from "@/components/app/use-current-game";
import { WriteLockNotice } from "@/components/app/write-lock-notice";
import { useSaves } from "@/data/source";
import { cn } from "@/lib/utils";
import { SaveDetail, saveFacts } from "./save-detail";

export function SavesScreen() {
  const { gameId } = useCurrentGame();
  const { data } = useSaves(gameId);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const saves = data?.saves ?? [];
  const skipped = data?.skipped ?? [];
  const selected =
    saves.find((save) => save.path === selectedPath) ?? saves[0] ?? null;

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Saves"
        description="Each save remembers the mods it was played with. Pick one to see which still match, and have Atelier set the mod list it needs."
      />
      <WriteLockNotice />
      {data && saves.length === 0 && skipped.length === 0 ? (
        <EmptyState icon={<SaveIcon />} title="No saves yet">
          Saves from Documents\Humankind\Save Files appear here once you play a
          game.
        </EmptyState>
      ) : (
        <div className="grid grid-cols-[minmax(0,1fr)] items-start gap-5 xl:grid-cols-[minmax(0,1fr)_28rem]">
          <div className="flex min-w-0 flex-col gap-5">
            <Panel title="Saves">
              <ul className="flex flex-col divide-y divide-line">
                {saves.map((save) => (
                  <li key={save.path}>
                    <button
                      type="button"
                      aria-current={save === selected ? "true" : undefined}
                      onClick={() => setSelectedPath(save.path)}
                      className={cn(
                        "-mx-2 flex w-[calc(100%+1rem)] flex-col rounded-field px-2 py-2.5 text-left transition-colors duration-(--dur-hover) hover:bg-surface",
                        save === selected && "bg-surface",
                      )}
                    >
                      <span className="text-ui font-semibold wrap-anywhere text-copy">
                        {save.name}
                      </span>
                      <span className="text-label text-copy-muted">
                        {saveFacts(save)}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            </Panel>
            {skipped.length > 0 && (
              <Panel title="Couldn't read">
                <ul className="flex flex-col gap-3">
                  {skipped.map((skip) => (
                    <li key={skip.path} className="flex gap-2">
                      <FileWarningIcon
                        aria-hidden="true"
                        className="mt-0.5 size-4 shrink-0 text-copy"
                      />
                      <div className="min-w-0">
                        <p className="font-mono text-label wrap-anywhere text-copy">
                          <BreakableText text={skip.path} />
                        </p>
                        <p className="text-ui text-copy-muted">
                          This file {skip.reason}. Humankind may still open it;
                          Atelier can't check its mods.
                        </p>
                      </div>
                    </li>
                  ))}
                </ul>
              </Panel>
            )}
          </div>
          {selected && <SaveDetail key={selected.path} save={selected} />}
        </div>
      )}
    </div>
  );
}
