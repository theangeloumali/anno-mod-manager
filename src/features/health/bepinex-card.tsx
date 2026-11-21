// Created by The Angelo Umali.
import { DownloadIcon, Trash2Icon } from "lucide-react";
import { withToast } from "@/components/app/notify";
import { Panel } from "@/components/app/panel";
import { StateBadge } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import { useAction, useBepinexStatus } from "@/data/source";
import type { BepInExStatus } from "@/data/types";

function statusBadge(status: BepInExStatus) {
  if (!status.present)
    return <StateBadge tone="neutral">Not installed</StateBadge>;
  const version = status.version ? ` ${status.version}` : "";
  return (
    <StateBadge tone="ok">
      {status.installedByAtelier
        ? `Installed by Atelier${version}`
        : `Installed by hand${version}`}
    </StateBadge>
  );
}

function PluginList({ status }: { status: BepInExStatus }) {
  if (!status.present)
    return (
      <p className="text-ui text-copy-muted">
        BepInEx loads plugins such as ModListGuard, which stops Humankind wiping
        the mod list when a module is missing.
      </p>
    );
  // Loaded plugins come from the last session's log, so no log means unknown, not none.
  if (!status.logFound)
    return (
      <p className="text-ui text-copy-muted">
        Humankind hasn't run with BepInEx yet, so the plugins it loads show here
        after the next launch.
      </p>
    );
  if (status.plugins.length === 0)
    return (
      <p className="text-ui text-copy-muted">
        The last session loaded no plugins from BepInEx\plugins.
      </p>
    );
  return (
    <ul className="flex flex-col divide-y divide-line">
      {status.plugins.map((plugin) => (
        <li
          key={plugin.name}
          className="flex flex-wrap justify-between gap-x-3 py-2 text-ui"
        >
          <span className="min-w-0 wrap-anywhere text-copy">{plugin.name}</span>
          <span className="font-mono text-label text-copy-dim">
            {plugin.version}
          </span>
        </li>
      ))}
    </ul>
  );
}

/** Humankind's plugin loader. Atelier only removes a copy it put there itself. */
export function BepinexCard() {
  const { gameId, isWriteLocked } = useCurrentGame();
  const { data: status } = useBepinexStatus(gameId);
  const install = useAction((source) => source.installBepinex(gameId));
  const uninstall = useAction((source) => source.uninstallBepinex(gameId));
  if (!status) return null;
  const isBusy = isWriteLocked || install.isPending || uninstall.isPending;

  return (
    <Panel title="BepInEx" actions={statusBadge(status)}>
      <PluginList status={status} />
      {status.present && status.logStale && (
        <p className="mt-2 text-label text-copy-muted">
          The BepInEx log is older than the last launch, so the plugins may not
          have loaded.
        </p>
      )}
      <div className="mt-4 flex flex-col items-start gap-2">
        {!status.present && (
          <Button
            size="sm"
            disabled={isBusy}
            onClick={() =>
              withToast(install.mutateAsync([]), "BepInEx installed")
            }
          >
            <DownloadIcon aria-hidden="true" />
            Install BepInEx
          </Button>
        )}
        {status.present && status.installedByAtelier && (
          <Button
            size="sm"
            variant="quiet"
            disabled={isBusy}
            onClick={() =>
              withToast(uninstall.mutateAsync([]), "BepInEx removed")
            }
          >
            <Trash2Icon aria-hidden="true" />
            Uninstall BepInEx
          </Button>
        )}
        {status.present && !status.installedByAtelier && (
          <p className="text-label text-copy-muted">
            You installed this copy yourself, so Atelier leaves it alone.
          </p>
        )}
        {isWriteLocked && (
          <p className="text-label text-copy-muted">
            Close Humankind to change BepInEx.
          </p>
        )}
      </div>
    </Panel>
  );
}
