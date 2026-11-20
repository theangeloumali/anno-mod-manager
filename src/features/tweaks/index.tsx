// Created by The Angelo Umali.
import { LockIcon, SlidersHorizontalIcon } from "lucide-react";
import { toast } from "sonner";
import { useAppStore } from "@/components/app/app-store";
import { ChoiceSelect } from "@/components/app/choice-select";
import { errorMessage } from "@/components/app/format";
import { isReadOnly } from "@/components/app/mod-badges";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState, Panel } from "@/components/app/panel";
import { useCurrentGame } from "@/components/app/use-current-game";
import { WriteLockNotice } from "@/components/app/write-lock-notice";
import { useAction, useModOptions, useMods } from "@/data/source";
import type { ModOption } from "@/data/types";
import { OptionField } from "./option-field";

export function TweaksScreen() {
  const { gameId, isWriteLocked } = useCurrentGame();
  const { data: mods = [] } = useMods(gameId);
  const tweaksModId = useAppStore((store) => store.tweaksModId);
  const openTweaks = useAppStore((store) => store.openTweaks);
  const tweakableMods = mods.filter((mod) => mod.hasOptions);
  const selectedMod =
    tweakableMods.find((mod) => mod.id === tweaksModId) ??
    tweakableMods.find((mod) => !isReadOnly(mod)) ??
    tweakableMods[0] ??
    null;
  const { data: options = [] } = useModOptions(gameId, selectedMod?.id ?? null);
  const setOption = useAction(
    (source, modId: string, key: string, value: ModOption["value"] | null) =>
      source.setOption(gameId, modId, key, value),
  );
  const isLocked =
    isWriteLocked || (selectedMod !== null && isReadOnly(selectedMod));

  const groups = [
    ...new Set(options.map((option) => option.group ?? "Options")),
  ];

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Tweaks"
        description="Settings that mods expose in their modinfo. Changes are saved as you make them and apply the next time the game starts."
      />
      <WriteLockNotice />
      {!selectedMod ? (
        <EmptyState
          icon={<SlidersHorizontalIcon />}
          title="No mods with options"
        >
          None of the installed mods expose settings. Mods that do show up here.
        </EmptyState>
      ) : (
        <>
          <ChoiceSelect
            label="Mod"
            className="sm:w-80"
            value={selectedMod.id}
            choices={tweakableMods.map((mod) => ({
              value: mod.id,
              label: mod.name,
            }))}
            onChange={openTweaks}
          />
          {isReadOnly(selectedMod) && (
            <p className="flex items-center gap-2 rounded-field border border-line bg-bg-elevated p-3 text-ui text-copy-muted">
              <LockIcon
                aria-hidden="true"
                className="size-4 shrink-0 text-accent"
              />
              {selectedMod.name} lives in the game folder, so its options are
              read-only here.
            </p>
          )}
          {groups.map((group) => (
            <Panel key={group} title={group}>
              <div className="divide-y divide-line">
                {options
                  .filter((option) => (option.group ?? "Options") === group)
                  .map((option) => (
                    <OptionField
                      key={`${option.key}:${String(option.value)}`}
                      option={option}
                      isDisabled={isLocked || setOption.isPending}
                      onChange={(value) =>
                        setOption.mutate([selectedMod.id, option.key, value], {
                          onError: (error) => toast.error(errorMessage(error)),
                        })
                      }
                    />
                  ))}
              </div>
            </Panel>
          ))}
        </>
      )}
    </div>
  );
}
