// Humankind's load order rules, as atelier-core's humankind/load_order.rs states them.
// Orders here leave out the base game, which always loads first.

export type ModuleKind = "Standalone" | "Conversion" | "Extension";

/** A blank category means the bundle couldn't be read; like the core, treat it as an Extension. */
export const moduleKind = (category: string): ModuleKind =>
  category === "Standalone" || category === "Conversion"
    ? category
    : "Extension";

type KindOf = (module: string) => ModuleKind;

export interface OrderGroups {
  standalone: string | null;
  conversion: string | null;
  /** The only modules the player can reorder. */
  extensions: string[];
}

export function splitOrder(order: string[], kindOf: KindOf): OrderGroups {
  return {
    standalone: order.find((name) => kindOf(name) === "Standalone") ?? null,
    conversion: order.find((name) => kindOf(name) === "Conversion") ?? null,
    extensions: order.filter((name) => kindOf(name) === "Extension"),
  };
}

export const joinOrder = (groups: OrderGroups): string[] => [
  ...(groups.standalone === null ? [] : [groups.standalone]),
  ...(groups.conversion === null ? [] : [groups.conversion]),
  ...groups.extensions,
];

/** A Standalone or Conversion replaces the one already on; the game allows one of each. */
export function enableModule(
  order: string[],
  module: string,
  kindOf: KindOf,
): string[] {
  if (order.includes(module)) return order;
  const kind = kindOf(module);
  if (kind === "Extension") return [...order, module];
  const groups = splitOrder(order, kindOf);
  return joinOrder(
    kind === "Standalone"
      ? { ...groups, standalone: module }
      : { ...groups, conversion: module },
  );
}

/** The first rule the order breaks, worded as the core words it, or null. */
export function orderProblem(order: string[], kindOf: KindOf): string | null {
  const standalones = order.filter((name) => kindOf(name) === "Standalone");
  const conversions = order.filter((name) => kindOf(name) === "Conversion");
  const [standalone, secondStandalone] = standalones;
  const [conversion, secondConversion] = conversions;
  if (standalone && secondStandalone)
    return `'${standalone}' and '${secondStandalone}' are both Standalone modules; only one can be on`;
  if (conversion && secondConversion)
    return `'${conversion}' and '${secondConversion}' are both Conversion modules; only one can be on`;
  if (standalone && order.indexOf(standalone) !== 0)
    return `Standalone module '${standalone}' must come right after the base game`;
  if (conversion && order.indexOf(conversion) !== standalones.length)
    return `Conversion module '${conversion}' must come right after the base game and any Standalone`;
  return null;
}

/** The extensions with one moved from `from` to `to`, or null when `to` is past either end. */
export function moveExtension(
  extensions: string[],
  from: number,
  to: number,
): string[] | null {
  const moved = extensions[from];
  if (moved === undefined || to < 0 || to >= extensions.length) return null;
  const rest = extensions.filter((_, index) => index !== from);
  return [...rest.slice(0, to), moved, ...rest.slice(to)];
}

/** Writes `extensions` into the order's Extension slots, leaving every other module where it is. */
export function withExtensions(
  order: string[],
  extensions: string[],
  kindOf: KindOf,
): string[] {
  const queue = [...extensions];
  return order.map((name) =>
    kindOf(name) === "Extension" ? (queue.shift() ?? name) : name,
  );
}
