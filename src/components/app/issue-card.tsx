// Created by The Angelo Umali.
import { DownloadIcon, ListOrderedIcon, PowerIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAction, type DataSource } from "@/data/source";
import type { GameId, Issue, IssueKind, Mod } from "@/data/types";
import { useAppStore } from "./app-store";
import { withToast } from "./notify";
import { StateIcon } from "./state-badge";

const KIND_LABEL: Record<IssueKind, string> = {
  "missing-dependency": "Missing dependency",
  "inactive-dependency": "Dependency turned off",
  incompatible: "Incompatible pair",
  deprecated: "Deprecated",
  dlc: "DLC required",
  "duplicate-id": "Duplicate ModID",
  "load-order-cycle": "LoadAfter loop",
};

/** mod.io is searched by ModID, then subscribed; the core installs it and its own dependencies. */
async function installDependencyFromModio(
  source: DataSource,
  game: GameId,
  dependencyId: string,
) {
  const page = await source.browseModio({
    game,
    search: dependencyId,
    tags: [],
    sort: "trending",
    subscribedOnly: false,
    offset: 0,
  });
  const match = page.items.find((item) => item.nameId === dependencyId);
  if (!match)
    throw new Error(
      `${dependencyId} isn't on mod.io. Install it from a file instead.`,
    );
  await source.subscribe(game, match.id);
}

export function IssueCard({
  issue,
  mods,
  gameId,
  isWriteLocked,
  showModName = true,
}: {
  issue: Issue;
  mods: Mod[];
  gameId: GameId;
  isWriteLocked: boolean;
  showModName?: boolean;
}) {
  const setView = useAppStore((store) => store.setView);
  const selectMod = useAppStore((store) => store.selectMod);
  const toggleMod = useAction((source, modId: string, active: boolean) =>
    source.setModActive(gameId, modId, active),
  );
  const installDependency = useAction((source, dependencyId: string) =>
    installDependencyFromModio(source, gameId, dependencyId),
  );
  const nameOf = (modId: string | null) =>
    mods.find((mod) => mod.id === modId)?.name ?? modId ?? "";
  const isBusy = toggleMod.isPending || installDependency.isPending;

  const turnOff = (modId: string) => (
    <Button
      key={modId}
      size="sm"
      disabled={isWriteLocked || isBusy}
      onClick={() =>
        withToast(
          toggleMod.mutateAsync([modId, false]),
          `Turned off ${nameOf(modId)}`,
        )
      }
    >
      <PowerIcon aria-hidden="true" />
      Turn off {nameOf(modId)}
    </Button>
  );

  const fixes: Record<IssueKind, React.ReactNode> = {
    "missing-dependency": issue.relatedModId && (
      <Button
        variant="steel"
        size="sm"
        disabled={isWriteLocked || isBusy}
        onClick={() =>
          withToast(
            installDependency.mutateAsync([issue.relatedModId ?? ""]),
            `Downloading ${issue.relatedModId} from mod.io`,
          )
        }
      >
        <DownloadIcon aria-hidden="true" />
        Install from mod.io
      </Button>
    ),
    "inactive-dependency": issue.relatedModId && (
      <Button
        variant="steel"
        size="sm"
        disabled={isWriteLocked || isBusy}
        onClick={() =>
          withToast(
            toggleMod.mutateAsync([issue.relatedModId ?? "", true]),
            `Turned on ${nameOf(issue.relatedModId)}`,
          )
        }
      >
        <PowerIcon aria-hidden="true" />
        Turn on {nameOf(issue.relatedModId)}
      </Button>
    ),
    incompatible: [
      turnOff(issue.modId),
      issue.relatedModId && turnOff(issue.relatedModId),
    ],
    deprecated: turnOff(issue.modId),
    dlc: turnOff(issue.modId),
    "duplicate-id": null,
    "load-order-cycle": (
      <Button size="sm" onClick={() => setView("load-order")}>
        <ListOrderedIcon aria-hidden="true" />
        See load order
      </Button>
    ),
  };

  return (
    <article className="flex gap-3 rounded-field border border-line bg-bg p-3">
      <StateIcon
        tone={issue.level === "error" ? "error" : "warning"}
        className="mt-1"
      />
      <div className="min-w-0 flex-1">
        <p className="text-label font-semibold tracking-wide text-copy-muted uppercase">
          {issue.level === "error" ? "Error" : "Warning"} ·{" "}
          {KIND_LABEL[issue.kind]}
        </p>
        {showModName && (
          <button
            type="button"
            className="mt-0.5 text-left text-ui font-semibold text-accent underline-offset-4 hover:underline"
            onClick={() => {
              selectMod(issue.modId);
              setView("library");
            }}
          >
            {nameOf(issue.modId)}
          </button>
        )}
        <p className="text-ui text-copy">{issue.message}</p>
        <div className="mt-2 flex flex-wrap gap-2">{fixes[issue.kind]}</div>
      </div>
    </article>
  );
}
