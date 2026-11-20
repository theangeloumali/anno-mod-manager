// Created by The Angelo Umali.
import { FlaskConicalIcon } from "lucide-react";
import { useState } from "react";
import { useAppStore } from "@/components/app/app-store";
import { ChoiceSelect } from "@/components/app/choice-select";
import { SWITCH_CLASS } from "@/components/app/field-class";
import { withToast } from "@/components/app/notify";
import { PageHeader } from "@/components/app/page-header";
import { Panel } from "@/components/app/panel";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useAction, useGames, useSettings } from "@/data/source";
import type { Settings } from "@/data/types";
import { AccountPanel } from "./account-panel";
import { ApiKeyPanel } from "./api-key-panel";
import { GamePaths } from "./game-paths";

const RETENTION_CHOICES = [
  { value: "7", label: "7 days" },
  { value: "30", label: "30 days" },
  { value: "90", label: "90 days" },
  { value: "forever", label: "Keep forever" },
];
type Theme = "dark" | "light";

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
          Your own API key lets Atelier browse mod.io; signing in adds
          subscriptions, ratings and collections. Installed mods work offline.
        </p>
        <div className="flex flex-col gap-3">
          <ApiKeyPanel />
          <AccountPanel />
        </div>
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
