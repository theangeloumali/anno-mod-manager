// Created by The Angelo Umali.
import { CornerDownRightIcon, LockIcon } from "lucide-react";
import { useAppStore } from "@/components/app/app-store";
import { SWITCH_CLASS } from "@/components/app/field-class";
import {
  categoryPrefix,
  isReadOnly,
  LocationBadge,
} from "@/components/app/mod-badges";
import { withToast } from "@/components/app/notify";
import { StateBadge } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useAction } from "@/data/source";
import type { Issue, Mod } from "@/data/types";
import { cn } from "@/lib/utils";
import { issuesForMod } from "./issues-for-mod";

function ModStatus({ mod, issues }: { mod: Mod; issues: Issue[] }) {
  const modIssues = issuesForMod(issues, mod.id);
  const errorCount = modIssues.filter(
    (issue) => issue.level === "error",
  ).length;
  if (errorCount)
    return (
      <StateBadge tone="error">
        {errorCount === 1 ? "Error" : `${errorCount} errors`}
      </StateBadge>
    );
  if (modIssues.length) return <StateBadge tone="warning">Warning</StateBadge>;
  if (mod.update)
    return <StateBadge tone="info">Update {mod.update.version}</StateBadge>;
  if (!mod.active) return <StateBadge tone="neutral">Off</StateBadge>;
  return <StateBadge tone="ok">Ready</StateBadge>;
}

function ModRow({
  mod,
  issues,
  isSubMod,
}: {
  mod: Mod;
  issues: Issue[];
  isSubMod: boolean;
}) {
  const { gameId, isWriteLocked } = useCurrentGame();
  const selectedModId = useAppStore((store) => store.selectedModId);
  const selectedCopy = useAppStore((store) => store.selectedModCopy);
  const selectMod = useAppStore((store) => store.selectMod);
  const toggle = useAction((source, active: boolean) =>
    source.setModActive(gameId, mod.id, active),
  );
  const isSelected =
    selectedModId === mod.id &&
    (!selectedCopy ||
      (selectedCopy.location === mod.location &&
        selectedCopy.folder === mod.folder));

  return (
    <TableRow
      data-state={isSelected ? "selected" : undefined}
      className={cn("border-line", isSelected && "bg-surface")}
    >
      <TableCell className="w-12 pl-3">
        <Switch
          className={SWITCH_CLASS}
          checked={mod.active}
          disabled={isWriteLocked || toggle.isPending}
          aria-label={`${mod.name} ${mod.active ? "on" : "off"}`}
          onCheckedChange={(checked) =>
            withToast(
              toggle.mutateAsync([checked]),
              `${mod.name} turned ${checked ? "on" : "off"}`,
            )
          }
        />
      </TableCell>
      <TableCell className="w-full max-w-0">
        <button
          type="button"
          aria-current={isSelected ? "true" : undefined}
          onClick={() =>
            selectMod(mod.id, { location: mod.location, folder: mod.folder })
          }
          title={`${mod.name} · ${mod.id}`}
          className={cn(
            "flex w-full min-w-0 items-center gap-2 text-left",
            isSubMod && "pl-5",
          )}
        >
          {isSubMod && (
            <CornerDownRightIcon
              aria-hidden="true"
              className="size-3.5 shrink-0 text-copy-dim"
            />
          )}
          <span className="min-w-0">
            <span className="block truncate text-ui font-semibold text-copy">
              <span className="font-normal text-copy-muted">
                {categoryPrefix(mod.category)}
              </span>
              {mod.name}
              {isSubMod && <span className="sr-only">, sub-mod</span>}
            </span>
            <span className="block truncate font-mono text-label text-copy-dim">
              {mod.id}
            </span>
          </span>
          {isReadOnly(mod) && (
            <LockIcon
              aria-label="Read-only"
              className="size-3.5 shrink-0 text-accent xl:hidden"
            />
          )}
        </button>
      </TableCell>
      <TableCell className="hidden font-mono text-label text-copy-muted tabular-nums sm:table-cell">
        {mod.version}
      </TableCell>
      <TableCell className="hidden 2xl:table-cell">
        <LocationBadge location={mod.location} />
      </TableCell>
      <TableCell className="pr-3 text-right">
        <ModStatus mod={mod} issues={issues} />
      </TableCell>
    </TableRow>
  );
}

/** One ModID can sit in two locations, so a sub-mod belongs to the parent copy in its own location. */
export const isSubModOf = (subMod: Mod, parent: Mod) =>
  subMod.parentId === parent.id && subMod.location === parent.location;

export function ModTable({
  mods,
  allMods,
  issues,
  isSubModVisible,
}: {
  mods: Mod[];
  allMods: Mod[];
  issues: Issue[];
  isSubModVisible: (subMod: Mod, parent: Mod) => boolean;
}) {
  return (
    <div className="min-w-0 overflow-hidden rounded-card border border-line bg-bg-elevated">
      <Table className="font-condensed">
        <TableHeader>
          <TableRow className="border-line hover:bg-transparent">
            <TableHead className="w-12 pl-3 text-label text-copy-dim">
              On
            </TableHead>
            <TableHead className="text-label text-copy-dim">Mod</TableHead>
            <TableHead className="hidden text-label text-copy-dim sm:table-cell">
              Version
            </TableHead>
            <TableHead className="hidden text-label text-copy-dim 2xl:table-cell">
              Location
            </TableHead>
            <TableHead className="pr-3 text-right text-label text-copy-dim">
              Status
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {mods.length === 0 && (
            <TableRow>
              <TableCell
                colSpan={5}
                className="py-8 text-center text-ui text-copy-muted"
              >
                No mods match this filter.
              </TableCell>
            </TableRow>
          )}
          {/* One ModID can sit in two locations, so a row is keyed by where its folder is. */}
          {mods.flatMap((mod) => [
            <ModRow
              key={`${mod.location}/${mod.folder}`}
              mod={mod}
              issues={issues}
              isSubMod={false}
            />,
            ...allMods
              .filter(
                (subMod) =>
                  isSubModOf(subMod, mod) && isSubModVisible(subMod, mod),
              )
              .map((subMod) => (
                <ModRow
                  key={`${subMod.location}/${subMod.folder}`}
                  mod={subMod}
                  issues={issues}
                  isSubMod
                />
              )),
          ])}
        </TableBody>
      </Table>
    </div>
  );
}
