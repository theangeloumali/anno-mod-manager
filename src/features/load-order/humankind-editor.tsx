// Created by The Angelo Umali.
import { RepeatIcon, SaveIcon } from "lucide-react";
import { useState } from "react";
import { withToast } from "@/components/app/notify";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState, Panel } from "@/components/app/panel";
import { useCurrentGame } from "@/components/app/use-current-game";
import { WriteLockNotice } from "@/components/app/write-lock-notice";
import { Button } from "@/components/ui/button";
import {
  moduleKind,
  modulesById,
  moveExtensionInOrder,
  orderProblem,
  splitOrder,
} from "@/data/humankind-order";
import { useAction, useLoadOrder, useMods } from "@/data/source";
import type { Mod } from "@/data/types";
import {
  MissingModulesAlert,
  OrderProblemAlert,
} from "./humankind-order-alerts";
import { MoveButtons, OrderRow } from "./humankind-order-row";

const EXTENSION_RULE =
  "Extensions load after the base game, the Standalone and the Conversion.";

export function HumankindOrderEditor() {
  const { gameId } = useCurrentGame();
  const { data: order } = useLoadOrder(gameId);
  const { data: mods } = useMods(gameId);
  const saved = order?.entries.map((entry) => entry.modId);
  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Load order"
        description="Humankind loads the base game, then one Standalone, then one Conversion, then Extensions in the order you set here."
      />
      <WriteLockNotice />
      {saved && mods && (
        // Remounts with the saved list, so a save or a profile switch resets the draft.
        <OrderDraft key={saved.join("\n")} saved={saved} mods={mods} />
      )}
    </div>
  );
}

function OrderDraft({ saved, mods }: { saved: string[]; mods: Mod[] }) {
  const { gameId, isWriteLocked } = useCurrentGame();
  const [draft, setDraft] = useState(saved);
  const [announcement, setAnnouncement] = useState("");
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const save = useAction((source, order: string[]) =>
    source.setLoadOrder(gameId, order),
  );
  const modById = modulesById(mods);
  const titleOf = (module: string) => modById.get(module)?.name ?? module;
  const kindOf = (module: string) =>
    moduleKind(modById.get(module)?.category ?? "");
  const groups = splitOrder(draft, kindOf);
  const missing = draft.filter((module) => !modById.has(module));
  const extraFixed = draft.filter(
    (module) =>
      kindOf(module) !== "Extension" &&
      module !== groups.standalone &&
      module !== groups.conversion,
  );
  const problem = orderProblem(draft, kindOf);
  const isDirty = draft.join("\n") !== saved.join("\n");
  const blockReason = missing.length
    ? "Remove the modules that aren't installed first."
    : problem;

  const moveTo = (from: number, to: number) => {
    const moved = groups.extensions[from] ?? "";
    const title = titleOf(moved);
    if (problem) {
      setAnnouncement(`Fix the list before moving ${title}: ${problem}.`);
      return;
    }
    const nextOrder = moveExtensionInOrder(draft, from, to, kindOf);
    if (!nextOrder) {
      setAnnouncement(
        to < from
          ? `${title} is already the first Extension. ${EXTENSION_RULE}`
          : `${title} is already the last Extension.`,
      );
      return;
    }
    setDraft(nextOrder);
    setAnnouncement(
      `${title} moved to position ${nextOrder.indexOf(moved) + 1} of ${nextOrder.length}.`,
    );
  };

  const remove = (module: string) => {
    setDraft(draft.filter((name) => name !== module));
    setAnnouncement(`${module} removed from the list. Save to write it.`);
  };

  if (saved.length === 0)
    return (
      <EmptyState icon={<RepeatIcon />} title="No mods are on">
        Turn mods on in the Library and they appear here. If Humankind hasn't
        run on this PC yet, start it once so it creates its mod list.
      </EmptyState>
    );

  const fixedRow = (module: string | null, kind: string, rule: string) =>
    module ? (
      <OrderRow
        position={draft.indexOf(module) + 1}
        title={titleOf(module)}
        note={rule}
      />
    ) : (
      <p className="py-2 text-ui text-copy-muted">No {kind} is on.</p>
    );

  return (
    <>
      {missing.length > 0 && (
        <MissingModulesAlert missing={missing} onRemove={remove} />
      )}
      {problem && <OrderProblemAlert problem={problem} />}

      <Panel title="Base game, Standalone and Conversion">
        <ol className="flex flex-col divide-y divide-line">
          <OrderRow position={0} title="Humankind" note="Always loads first" />
          {fixedRow(
            groups.standalone,
            "Standalone",
            "Standalone · right after the base game",
          )}
          {fixedRow(
            groups.conversion,
            "Conversion",
            "Conversion · right after any Standalone",
          )}
          {extraFixed.map((module) => (
            <OrderRow
              key={module}
              position={draft.indexOf(module) + 1}
              title={titleOf(module)}
              note={`${kindOf(module)} · only one can be on`}
            >
              <Button size="sm" onClick={() => remove(module)}>
                Remove from list
              </Button>
            </OrderRow>
          ))}
        </ol>
      </Panel>

      <Panel
        title="Extensions"
        actions={
          <span className="text-label text-copy-dim">
            Drag, or use the arrows. They load top to bottom.
          </span>
        }
      >
        <p role="status" className="mb-2 min-h-5 text-ui text-copy-muted">
          {announcement}
        </p>
        {groups.extensions.length === 0 ? (
          <p className="text-ui text-copy-muted">No Extensions are on.</p>
        ) : (
          <ol className="flex flex-col divide-y divide-line">
            {groups.extensions.map((module, index) => (
              <OrderRow
                key={module}
                position={draft.indexOf(module) + 1}
                title={titleOf(module)}
                note={modById.has(module) ? "Extension" : "Not installed"}
                isMissing={!modById.has(module)}
                isDragging={dragIndex === index}
                drag={{
                  onDragStart: () => setDragIndex(index),
                  onDragEnd: () => setDragIndex(null),
                  onDrop: () =>
                    dragIndex !== null &&
                    dragIndex !== index &&
                    moveTo(dragIndex, index),
                }}
              >
                <MoveButtons
                  title={titleOf(module)}
                  isFirst={index === 0}
                  isLast={index === groups.extensions.length - 1}
                  onMove={(delta) => moveTo(index, index + delta)}
                />
              </OrderRow>
            ))}
          </ol>
        )}
      </Panel>

      <div className="flex flex-wrap items-center gap-3">
        <Button
          variant="primary"
          disabled={
            !isDirty || isWriteLocked || blockReason !== null || save.isPending
          }
          aria-describedby="save-order-reason"
          onClick={() =>
            withToast(
              save.mutateAsync([draft]),
              "Load order saved to Registry.xml",
            )
          }
        >
          <SaveIcon aria-hidden="true" data-icon="inline-start" />
          Save load order
        </Button>
        {isDirty && (
          <Button variant="quiet" onClick={() => setDraft(saved)}>
            Discard changes
          </Button>
        )}
        <p id="save-order-reason" className="text-ui text-copy-muted">
          {isWriteLocked
            ? "Close Humankind to save."
            : (blockReason ?? (isDirty ? "Unsaved changes." : "Saved."))}
        </p>
      </div>
    </>
  );
}
