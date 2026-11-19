// Created by The Angelo Umali.
import {
  CircleCheckIcon,
  FolderPlusIcon,
  InfoIcon,
  LockIcon,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";
import { useAppStore } from "@/components/app/app-store";
import { SWITCH_CLASS } from "@/components/app/field-class";
import { StateBadge } from "@/components/app/state-badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { useGames, useMods } from "@/data/source";
import type { GameId } from "@/data/types";
import { cn } from "@/lib/utils";
import { AccountPanel } from "../settings/account-panel";
import { ApiKeyPanel } from "../settings/api-key-panel";

const STEP_TITLES = [
  "Welcome to Atelier",
  "Where should mods go?",
  "Connect mod.io",
  "You're set",
];

/** Mounted only on its step, so the closed wizard never queries the other game's mods. */
function InstalledModCount() {
  const { data: mods117 = [] } = useMods("anno117");
  const { data: mods1800 = [] } = useMods("anno1800");
  return <>{mods117.length + mods1800.length}</>;
}

function Callout({ icon, children }: { icon: ReactNode; children: ReactNode }) {
  return (
    <p className="flex gap-2 rounded-field border border-line bg-bg p-3 text-ui text-copy-muted [&_svg]:mt-0.5 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-accent">
      {icon}
      <span>{children}</span>
    </p>
  );
}

export function FirstRunWizard() {
  const isOpen = useAppStore((store) => store.isWizardOpen);
  const setOpen = useAppStore((store) => store.setWizardOpen);
  const { data: games = [] } = useGames();
  const [stepIndex, setStepIndex] = useState(0);
  const [managedGames, setManagedGames] = useState<Record<GameId, boolean>>({
    anno117: true,
    anno1800: true,
  });
  const isLastStep = stepIndex === STEP_TITLES.length - 1;

  const close = () => {
    setOpen(false);
    setStepIndex(0);
  };

  const steps: ReactNode[] = [
    <>
      <p className="text-ui text-copy-muted">
        We found these games on this PC. Pick the ones to manage. You can add
        more later.
      </p>
      <ul className="flex flex-col gap-2">
        {games.map((game) => (
          <li
            key={game.id}
            className="flex items-center gap-3 rounded-field border border-line bg-bg p-3"
          >
            <CircleCheckIcon
              aria-hidden="true"
              className="size-5 shrink-0 text-success"
            />
            <div className="min-w-0 flex-1">
              <p className="text-ui font-semibold">{game.displayName}</p>
              <p className="truncate font-mono text-label text-copy-dim">
                Steam · {game.installPath}
              </p>
            </div>
            <Switch
              className={SWITCH_CLASS}
              aria-label={`Manage ${game.displayName}`}
              checked={managedGames[game.id]}
              onCheckedChange={(checked) =>
                setManagedGames((current) => ({
                  ...current,
                  [game.id]: checked,
                }))
              }
            />
          </li>
        ))}
      </ul>
      <Button
        variant="quiet"
        className="self-start"
        onClick={() =>
          toast.info("The folder picker opens in the desktop app.")
        }
      >
        <FolderPlusIcon aria-hidden="true" />
        Add a game folder by hand
      </Button>
    </>,
    <>
      <p className="text-ui text-copy-muted">
        Documents folders need no admin rights and survive game reinstalls, so
        Atelier installs there.
      </p>
      <ul className="flex flex-col gap-2">
        {games
          .filter((game) => managedGames[game.id])
          .map((game) => (
            <li
              key={game.id}
              className="flex flex-wrap items-center gap-3 rounded-field border border-line bg-bg p-3"
            >
              <div className="min-w-0 flex-1">
                <p className="text-ui font-semibold">{game.displayName}</p>
                <p className="truncate font-mono text-label text-copy-dim">
                  {game.modsPath}
                </p>
              </div>
              <StateBadge tone="ok">Writable</StateBadge>
            </li>
          ))}
      </ul>
      <Callout icon={<InfoIcon />}>
        Atelier found <InstalledModCount /> mods already installed, including
        ones in the game folders, and keeps their on/off state.
      </Callout>
    </>,
    <>
      <p className="text-ui text-copy-muted">
        Add your own mod.io API key to browse mod.io. Signing in is optional: it
        lets you subscribe, rate and sync. You can skip both and still install
        mods you drop in.
      </p>
      <ApiKeyPanel />
      <AccountPanel />
    </>,
    <>
      <p className="text-ui text-copy-muted">
        Drop any mod archive on the window to install it, or browse mod.io.
        Atelier checks every change before you launch.
      </p>
      <Callout icon={<LockIcon />}>
        While a game is running, Atelier locks its mods folder so nothing
        changes mid-session.
      </Callout>
    </>,
  ];

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => (open ? setOpen(true) : close())}
    >
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto bg-bg-elevated p-6 sm:max-w-lg">
        <ol
          className="flex gap-1.5"
          aria-label={`Step ${stepIndex + 1} of ${STEP_TITLES.length}`}
        >
          {STEP_TITLES.map((title, index) => (
            <li
              key={title}
              aria-current={index === stepIndex ? "step" : undefined}
              className={cn(
                "h-1 flex-1 rounded-pill",
                index <= stepIndex ? "bg-accent" : "bg-surface",
              )}
            >
              <span className="sr-only">
                {title}
                {index < stepIndex ? " (done)" : ""}
              </span>
            </li>
          ))}
        </ol>
        <DialogHeader>
          <DialogTitle className="font-display text-h3 font-bold">
            {STEP_TITLES[stepIndex]}
          </DialogTitle>
          <DialogDescription className="text-label text-copy-dim">
            Step {stepIndex + 1} of {STEP_TITLES.length}
          </DialogDescription>
        </DialogHeader>
        {/* min-w-0: a grid child would otherwise widen to the unwrapped path and scroll the dialog. */}
        <div className="flex min-w-0 flex-col gap-3">{steps[stepIndex]}</div>
        <DialogFooter className="flex-row justify-end bg-transparent">
          {stepIndex > 0 && (
            <Button
              variant="quiet"
              className="mr-auto"
              onClick={() => setStepIndex(stepIndex - 1)}
            >
              Back
            </Button>
          )}
          {stepIndex === 2 && (
            <Button variant="quiet" onClick={() => setStepIndex(3)}>
              Skip
            </Button>
          )}
          <Button
            variant="steel"
            onClick={() => (isLastStep ? close() : setStepIndex(stepIndex + 1))}
          >
            {isLastStep ? "Start using Atelier" : "Continue"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
