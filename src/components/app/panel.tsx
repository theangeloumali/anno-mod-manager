// Created by The Angelo Umali.
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** A titled section: elevation is a border, not a shadow (tokens.css). */
export function Panel({
  title,
  actions,
  children,
  className,
}: {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "min-w-0 rounded-card border border-line bg-bg-elevated p-4 md:p-5",
        className,
      )}
    >
      {(title || actions) && (
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          {title && (
            <h2 className="text-ui-lg font-semibold text-copy">{title}</h2>
          )}
          {actions}
        </div>
      )}
      {children}
    </section>
  );
}

export function EmptyState({
  icon,
  title,
  children,
  actions,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-3 py-12 text-center">
      <div className="grid size-14 place-items-center rounded-full border border-line-strong text-accent [&_svg]:size-6">
        {icon}
      </div>
      <h2 className="text-panel-title font-semibold">{title}</h2>
      <p className="text-ui text-copy-muted">{children}</p>
      {actions && (
        <div className="mt-2 flex flex-wrap justify-center gap-2">
          {actions}
        </div>
      )}
    </div>
  );
}
