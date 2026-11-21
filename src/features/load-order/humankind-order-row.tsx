// Created by The Angelo Umali.
import { ArrowDownIcon, ArrowUpIcon, GripVerticalIcon } from "lucide-react";
import type { DragEvent, ReactNode } from "react";
import { StateBadge } from "@/components/app/state-badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/** One module in the order. Fixed rows pass no `children`; extensions pass their move controls. */
export function OrderRow({
  position,
  title,
  note,
  isMissing = false,
  isDragging = false,
  drag,
  children,
}: {
  position: number;
  title: string;
  note: string;
  isMissing?: boolean;
  isDragging?: boolean;
  drag?: {
    onDragStart: () => void;
    onDragEnd: () => void;
    onDrop: () => void;
  };
  children?: ReactNode;
}) {
  const allowDrop = (event: DragEvent) => event.preventDefault();
  return (
    <li
      draggable={drag !== undefined}
      onDragStart={(event) => {
        event.dataTransfer.setData("text/plain", title);
        drag?.onDragStart();
      }}
      onDragEnd={drag?.onDragEnd}
      onDragOver={allowDrop}
      onDrop={(event) => {
        event.preventDefault();
        drag?.onDrop();
      }}
      className={cn(
        "flex items-center gap-2 py-2",
        isDragging && "opacity-50",
        isMissing && "-mx-2 rounded-field bg-danger/10 px-2",
      )}
    >
      {drag ? (
        <GripVerticalIcon
          aria-hidden="true"
          className="hidden size-4 shrink-0 cursor-grab text-copy-dim sm:block"
        />
      ) : (
        <span aria-hidden="true" className="hidden w-4 shrink-0 sm:block" />
      )}
      <span className="w-6 shrink-0 font-mono text-label text-copy-dim tabular-nums">
        {String(position).padStart(2, "0")}
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-ui font-semibold text-copy" title={title}>
          {title}
        </p>
        <p className="text-label text-copy-muted">{note}</p>
      </div>
      {isMissing && <StateBadge tone="error">Not installed</StateBadge>}
      {children}
    </li>
  );
}

/** Up/down stay focusable at the ends so a refused move can explain itself. */
export function MoveButtons({
  title,
  isFirst,
  isLast,
  onMove,
}: {
  title: string;
  isFirst: boolean;
  isLast: boolean;
  onMove: (delta: -1 | 1) => void;
}) {
  const controls = [
    { delta: -1, label: "up", icon: ArrowUpIcon, isAtEnd: isFirst },
    { delta: 1, label: "down", icon: ArrowDownIcon, isAtEnd: isLast },
  ] as const;
  return (
    <div className="flex shrink-0">
      {controls.map(({ delta, label, icon: Icon, isAtEnd }) => (
        <Button
          key={label}
          variant="quiet"
          size="icon"
          aria-label={`Move ${title} ${label}`}
          aria-disabled={isAtEnd || undefined}
          className={cn(isAtEnd && "opacity-40")}
          onClick={() => onMove(delta)}
        >
          <Icon aria-hidden="true" data-icon="inline-start" />
        </Button>
      ))}
    </div>
  );
}
