// Created by The Angelo Umali.
// Same field look as ZKidz Studio (apps/web/src/components/field-class.ts), so forms read as one system.
export const FIELD_CLASS =
  "min-h-(--target-min) w-full rounded-field border border-line-strong bg-bg px-3 text-ui text-copy placeholder:text-copy-dim " +
  "transition-[border-color] duration-(--dur-1) ease-out hover:border-copy-dim disabled:cursor-not-allowed disabled:opacity-50";

/** shadcn's SelectTrigger sized and coloured to match FIELD_CLASS. */
export const SELECT_TRIGGER_CLASS =
  "min-h-(--target-min) w-full rounded-field border-line-strong bg-bg text-ui text-copy dark:bg-bg dark:hover:bg-surface";

/** Steel instead of gold: gold is reserved for the one primary action per view. */
export const SWITCH_CLASS = "data-checked:bg-accent";
