// Created by The Angelo Umali.
import { FlaskConicalIcon, FolderOpenIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { useAppStore } from "@/components/app/app-store";
import { ChoiceSelect } from "@/components/app/choice-select";
import { FIELD_CLASS, SWITCH_CLASS } from "@/components/app/field-class";
import { withToast } from "@/components/app/notify";
import { PageHeader } from "@/components/app/page-header";
import { Panel } from "@/components/app/panel";
import { StateBadge } from "@/components/app/state-badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useAction, useGames, useSettings } from "@/data/source";
import type { GameInfo, Settings, Store } from "@/data/types";
import { AccountPanel } from "./account-panel";

const STORE_LABEL: Record<Store, string> = {
  steam: "Steam",
  ubisoft: "Ubisoft Connect",
  epic: "Epic Games",
  manual: "Added by hand",
};
const RETENTION_CHOICES = [
  { value: "7", label: "7 days" },
  { value: "30", label: "30 days" },
  { value: "90", label: "90 days" },
  { value: "forever", label: "Keep forever" },
];
type Theme = "dark" | "light";

function PathField({ label, value }: { label: string; value: string }) {
  return (
    <label className="grid gap-1.5 text-label font-medium text-copy-muted">
      {label}
      <span className="flex gap-2">
        <input
          readOnly
          value={value}
          className={`${FIELD_CLASS} font-mono text-label`}
        />
        {/* No path setter in the contract yet (report DEVIATIONS); the picker belongs to Tauri's dialog API. */}
        <Button
          className="shrink-0"
          onClick={() =>
            toast.info("The folder picker opens in the desktop app.")
          }
        >
          <FolderOpenIcon aria-hidden="true" />
          <span className="sr-only sm:not-sr-only">Browse</span>
        </Button>
      </span>
    </label>
  );
}

function GamePaths({ game }: { game: GameInfo }) {
  return (
    <div className="flex flex-col gap-3 rounded-field border border-line bg-bg p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-ui font-semibold">{game.displayName}</h3>
        {game.store ? (
          <StateBadge tone="ok">{STORE_LABEL[game.store]}</StateBadge>
        ) : (
          <StateBadge tone="warning">Not found</StateBadge>
        )}
      </div>
      <PathField
        label="Game folder"
        value={game.installPath ?? "Not detected"}
      />
      <PathField
        label="Mods folder (Atelier writes here)"
        value={game.modsPath}
      />
      <PathField label="Mod loader log" value={game.logPath} />
    </div>
  );
}

export function SettingsScreen() {
  const { data: games = [] } = useGames();
  const { data: settings } = useSettings();
  const setWizardOpen = useAppStore((store) => store.setWizardOpen);
  const save = useAction((source, nextSettings: Settings) =>
    source.saveSettings(nextSettings),
  );
  const [theme, setTheme] = useState<Theme>(
    document.documentElement.dataset.theme === "light" ? "light" : "dark",
  );

  const update = (patch: Partial<Settings>) => {
    if (!settings) return;
    void withToast(
      save.mutateAsync([{ ...settings, ...patch }]),
      "Settings saved",
    );
  };

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Settings"
        description="Paths are detected automatically. Change them only if Atelier picked the wrong install."
        actions={
          <Button onClick={() => setWizardOpen(true)}>
            <FlaskConicalIcon aria-hidden="true" />
            Run setup again
          </Button>
        }
      />
      <Panel title="mod.io account">
        <p className="mb-3 text-ui text-copy-muted">
          Used for subscriptions, ratings and collections. Atelier works offline
          with installed mods.
        </p>
        <AccountPanel />
      </Panel>
      <Panel title="Games">
        <p className="mb-3 text-ui text-copy-muted">
          Detected from Steam library folders, Ubisoft Connect registry entries
          and Epic manifests.
        </p>
        <div className="grid gap-3 xl:grid-cols-2">
          {games.map((game) => (
            <GamePaths key={game.id} game={game} />
          ))}
        </div>
      </Panel>
      {settings && (
        <Panel title="Install behaviour">
          <div className="grid gap-4 md:grid-cols-2">
            <ChoiceSelect
              label="New mods start"
              value={settings.newModsStart}
              choices={[
                { value: "active", label: "Turned on" },
                { value: "inactive", label: "Turned off" },
                { value: "profile", label: "As the active profile says" },
              ]}
              onChange={(newModsStart) => update({ newModsStart })}
            />
            <ChoiceSelect
              label="Check for updates"
              value={settings.updateCheck}
              choices={[
                { value: "startup", label: "When Atelier starts" },
                { value: "hourly", label: "Every hour" },
                { value: "manual", label: "Only when I ask" },
              ]}
              onChange={(updateCheck) => update({ updateCheck })}
            />
            <ChoiceSelect
              label="Keep replaced versions"
              value={
                settings.backupRetentionDays === null
                  ? "forever"
                  : String(settings.backupRetentionDays)
              }
              choices={RETENTION_CHOICES}
              onChange={(retention) =>
                update({
                  backupRetentionDays:
                    retention === "forever" ? null : Number(retention),
                })
              }
            />
            <label className="flex items-center justify-between gap-3 self-end rounded-field border border-line bg-bg p-3 text-ui">
              <span>
                Install required dependencies automatically
                <span className="block text-label text-copy-muted">
                  From mod.io, when a mod you add needs them
                </span>
              </span>
              <Switch
                className={SWITCH_CLASS}
                checked={settings.installDependencies}
                onCheckedChange={(installDependencies) =>
                  update({ installDependencies })
                }
              />
            </label>
          </div>
        </Panel>
      )}
      {settings && (
        <Panel title="App">
          <div className="grid gap-4 md:grid-cols-3">
            <ChoiceSelect
              label="Language"
              value={settings.language}
              choices={[
                { value: "en", label: "English" },
                { value: "de", label: "Deutsch" },
                { value: "fr", label: "Français" },
              ]}
              onChange={(language) => update({ language })}
            />
            <ChoiceSelect
              label="Update channel"
              value={settings.updateChannel}
              choices={[
                { value: "stable", label: "Stable" },
                { value: "beta", label: "Beta" },
              ]}
              onChange={(updateChannel) => update({ updateChannel })}
            />
            <ChoiceSelect<Theme>
              label="Theme"
              value={theme}
              choices={[
                { value: "dark", label: "Navy (dark)" },
                { value: "light", label: "Cream (light)" },
              ]}
              onChange={(nextTheme) => {
                document.documentElement.dataset.theme = nextTheme;
                setTheme(nextTheme);
              }}
            />
          </div>
        </Panel>
      )}
    </div>
  );
}
