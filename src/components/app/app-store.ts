// Created by The Angelo Umali.
// UI-only state (which screen, which game, which sheet is open). Mod data stays in React Query.
import { create } from "zustand";
import type { GameId, InstallPreview } from "@/data/types";

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
  tweaksModId: string | null;
  preview: InstallPreview | null;
  isWizardOpen: boolean;
  isMobileNavOpen: boolean;
  setGame: (game: GameId) => void;
  setView: (view: ViewId) => void;
  selectMod: (modId: string | null) => void;
  openTweaks: (modId: string) => void;
  showPreview: (preview: InstallPreview | null) => void;
  setWizardOpen: (isOpen: boolean) => void;
  setMobileNavOpen: (isOpen: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  game: "anno117",
  view: "library",
  selectedModId: null,
  tweaksModId: null,
  preview: null,
  isWizardOpen: false,
  isMobileNavOpen: false,
  setGame: (game) => set({ game, selectedModId: null, tweaksModId: null }),
  setView: (view) => set({ view, isMobileNavOpen: false }),
  selectMod: (selectedModId) => set({ selectedModId }),
  openTweaks: (tweaksModId) => set({ tweaksModId, view: "tweaks" }),
  showPreview: (preview) => set({ preview }),
  setWizardOpen: (isWizardOpen) => set({ isWizardOpen }),
  setMobileNavOpen: (isMobileNavOpen) => set({ isMobileNavOpen }),
}));
