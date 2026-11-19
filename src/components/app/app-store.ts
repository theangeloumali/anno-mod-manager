// Created by The Angelo Umali.
// UI-only state (which screen, which game, which sheet is open). Mod data stays in React Query.
import { create } from "zustand";
import type { GameId, InstallPreview, Mod } from "@/data/types";

/** Which copy a row click picked: one ModID can sit in the game folder and in Documents. */
export type ModCopy = Pick<Mod, "location" | "folder">;

/** The clicked copy, else the last one scanned, which is the copy the game loads. */
export function findSelectedMod(
  mods: Mod[],
  modId: string | null,
  copy: ModCopy | null,
): Mod | null {
  return (
    mods.findLast(
      (mod) =>
        mod.id === modId &&
        (!copy ||
          (mod.location === copy.location && mod.folder === copy.folder)),
    ) ?? null
  );
}

export type ViewId =
  | "library"
  | "load-order"
  | "tweaks"
  | "browse"
  | "collections"
  | "downloads"
  | "health"
  | "settings";

interface AppState {
  game: GameId;
  view: ViewId;
  selectedModId: string | null;
  selectedModCopy: ModCopy | null;
  tweaksModId: string | null;
  preview: InstallPreview | null;
  isWizardOpen: boolean;
  isMobileNavOpen: boolean;
  setGame: (game: GameId) => void;
  setView: (view: ViewId) => void;
  /** Without `copy` the copy the game loads is shown (links from issues and load order). */
  selectMod: (modId: string | null, copy?: ModCopy) => void;
  openTweaks: (modId: string) => void;
  showPreview: (preview: InstallPreview | null) => void;
  setWizardOpen: (isOpen: boolean) => void;
  setMobileNavOpen: (isOpen: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  game: "anno117",
  view: "library",
  selectedModId: null,
  selectedModCopy: null,
  tweaksModId: null,
  preview: null,
  isWizardOpen: false,
  isMobileNavOpen: false,
  setGame: (game) =>
    set({
      game,
      selectedModId: null,
      selectedModCopy: null,
      tweaksModId: null,
    }),
  setView: (view) => set({ view, isMobileNavOpen: false }),
  selectMod: (selectedModId, copy) =>
    set({ selectedModId, selectedModCopy: copy ?? null }),
  openTweaks: (tweaksModId) => set({ tweaksModId, view: "tweaks" }),
  showPreview: (preview) => set({ preview }),
  setWizardOpen: (isWizardOpen) => set({ isWizardOpen }),
  setMobileNavOpen: (isMobileNavOpen) => set({ isMobileNavOpen }),
}));
