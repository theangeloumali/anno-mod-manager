// Created by The Angelo Umali.
import { ClipboardCopyIcon, SaveIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { ChoiceSelect } from "@/components/app/choice-select";
import { FIELD_CLASS } from "@/components/app/field-class";
import { errorMessage } from "@/components/app/format";
import { withToast } from "@/components/app/notify";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useAction, useProfiles } from "@/data/source";

const PROFILE_FILE_HINT = {
  profile: "mods\\profile.txt → profiles\\<name>.txt",
  legacy: "mods\\active-profile.txt",
  "activation-json": "mods\\activation.json",
} as const;

export function ProfileBar() {
  const { gameId, game, isWriteLocked } = useCurrentGame();
  const { data: profiles = [] } = useProfiles(gameId);
  const [isSaveOpen, setSaveOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const switchProfile = useAction((source, name: string) =>
    source.switchProfile(gameId, name),
  );
  const saveProfile = useAction((source, name: string) =>
    source.saveProfile(gameId, name),
  );
  const deleteProfile = useAction((source, name: string) =>
    source.deleteProfile(gameId, name),
  );
  const exportProfile = useAction((source, name: string) =>
    source.exportProfile(gameId, name),
  );
  const activeProfile = profiles.find((profile) => profile.active);

  const copyExport = async () => {
    if (!activeProfile) return;
    try {
      const text = await exportProfile.mutateAsync([activeProfile.name]);
      await navigator.clipboard.writeText(text);
      toast.success(`Copied “${activeProfile.name}” to the clipboard`);
    } catch (error) {
      toast.error(errorMessage(error));
    }
  };

  return (
    <div className="flex flex-col gap-3 rounded-card border border-line bg-bg-elevated p-4 md:flex-row md:items-end">
      <ChoiceSelect
        label="Profile"
        className="md:w-64"
        value={activeProfile?.name ?? ""}
        isDisabled={isWriteLocked}
        choices={profiles.map((profile) => ({
          value: profile.name,
          label: profile.builtIn ? `${profile.name} (built in)` : profile.name,
        }))}
        onChange={(name) =>
          withToast(switchProfile.mutateAsync([name]), `Switched to ${name}`)
        }
      />
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          disabled={isWriteLocked}
          onClick={() => setSaveOpen(true)}
        >
          <SaveIcon aria-hidden="true" />
          Save as…
        </Button>
        <Button size="sm" onClick={() => void copyExport()}>
          <ClipboardCopyIcon aria-hidden="true" />
          Export
        </Button>
        {activeProfile && !activeProfile.builtIn && (
          <Button
            size="sm"
            variant="quiet"
            disabled={isWriteLocked}
            onClick={() =>
              withToast(
                deleteProfile.mutateAsync([activeProfile.name]),
                `Deleted ${activeProfile.name}`,
              )
            }
          >
            <Trash2Icon aria-hidden="true" />
            Delete
          </Button>
        )}
      </div>
      {game && (
        <p className="text-label text-copy-dim md:ml-auto md:text-right">
          Written to{" "}
          <span className="font-mono">
            {PROFILE_FILE_HINT[game.activationScheme]}
          </span>
        </p>
      )}

      <Dialog open={isSaveOpen} onOpenChange={setSaveOpen}>
        <DialogContent className="bg-bg-elevated sm:max-w-md">
          <form
            className="grid gap-4"
            onSubmit={async (event) => {
              event.preventDefault();
              const name = newName.trim();
              if (!name) return;
              const result = await withToast(
                saveProfile.mutateAsync([name]),
                `Saved profile ${name}`,
              );
              if (result !== undefined) {
                setSaveOpen(false);
                setNewName("");
              }
            }}
          >
            <DialogHeader>
              <DialogTitle className="text-panel-title font-semibold">
                Save the current set
              </DialogTitle>
              <DialogDescription className="text-ui text-copy-muted">
                Saves which mods are on right now, so you can switch back in one
                click.
              </DialogDescription>
            </DialogHeader>
            <label className="grid gap-1.5 text-label font-medium text-copy-muted">
              Profile name
              <input
                required
                autoFocus
                value={newName}
                onChange={(event) => setNewName(event.target.value)}
                className={FIELD_CLASS}
                placeholder="Multiplayer Saturday"
              />
            </label>
            <DialogFooter className="bg-transparent">
              <DialogClose render={<Button type="button" variant="quiet" />}>
                Cancel
              </DialogClose>
              <Button type="submit" variant="steel" disabled={!newName.trim()}>
                Save profile
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
