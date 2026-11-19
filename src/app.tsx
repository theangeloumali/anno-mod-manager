// Created by The Angelo Umali.
import { isTauri } from "@tauri-apps/api/core";
import { useEffect, type ComponentType, type CSSProperties } from "react";
import { useAppStore, type ViewId } from "@/components/app/app-store";
import { DropOverlay } from "@/components/app/drop-overlay";
import { InstallPreviewSheet } from "@/components/app/install-preview-sheet";
import { MockStatesMenu } from "@/components/app/mock-states-menu";
import { Shell } from "@/components/app/shell";
import { Toaster } from "@/components/ui/sonner";
import { BrowseScreen } from "@/features/browse";
import { CollectionsScreen } from "@/features/collections";
import { DownloadsScreen } from "@/features/downloads";
import { HealthScreen } from "@/features/health";
import { LibraryScreen } from "@/features/library";
import { LoadOrderScreen } from "@/features/load-order";
import { FirstRunWizard } from "@/features/onboarding/first-run-wizard";
import { SettingsScreen } from "@/features/settings";
import { TweaksScreen } from "@/features/tweaks";

const SCREENS: Record<ViewId, ComponentType> = {
  library: LibraryScreen,
  "load-order": LoadOrderScreen,
  tweaks: TweaksScreen,
  browse: BrowseScreen,
  collections: CollectionsScreen,
  downloads: DownloadsScreen,
  health: HealthScreen,
  settings: SettingsScreen,
};

/** sonner reads its own variables; point them at the ZKidz tokens. */
const TOAST_STYLE = {
  "--normal-bg": "var(--color-bg-elevated)",
  "--normal-text": "var(--color-copy)",
  "--normal-border": "var(--color-border-strong)",
  "--border-radius": "var(--radius-field)",
} as CSSProperties;

export function App() {
  const view = useAppStore((store) => store.view);
  const Screen = SCREENS[view];

  // shadcn's dark: variant keys off data-theme, so the default ground must set it explicitly.
  useEffect(() => {
    document.documentElement.dataset.theme ??= "dark";
  }, []);

  return (
    <>
      <Shell>
        <Screen />
      </Shell>
      <DropOverlay />
      <InstallPreviewSheet />
      <FirstRunWizard />
      {!isTauri() && <MockStatesMenu />}
      {/* Lifted clear of the status bar, which wraps to two lines on phones. */}
      <Toaster
        position="bottom-left"
        offset={{ bottom: "3rem" }}
        mobileOffset={{ bottom: "5.5rem" }}
        style={TOAST_STYLE}
      />
    </>
  );
}
