// Created by The Angelo Umali.
import type { ReactNode } from "react";

/** The one h1 per screen; the shell's main region is labelled by it. */
export function PageHeader({
  title,
  description,
  actions,
}: {
  title: string;
  description: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="flex flex-col gap-4 border-b border-line pb-5 md:flex-row md:items-end md:justify-between">
      <div className="min-w-0">
        <h1 id="page-title" className="font-display text-h3 font-bold">
          {title}
        </h1>
        <p className="mt-1 max-w-[65ch] text-ui text-copy-muted">
          {description}
        </p>
      </div>
      {actions && (
        <div className="flex flex-wrap items-center gap-2">{actions}</div>
      )}
    </header>
  );
}
