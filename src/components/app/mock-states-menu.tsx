// Created by The Angelo Umali.
// Mock-up only: the one place allowed to reach into the mock source, to flip edge states.
import { useQueryClient } from "@tanstack/react-query";
import { FlaskConicalIcon } from "lucide-react";
import { useState } from "react";
import { buttonVariants } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  MOCK_DROP_SOURCES,
  mockScenario,
  rememberMockGame,
  resetMockData,
  restartSampleDownloads,
  setMockScenario,
  type MockScenario,
} from "@/data/mock-source";
import { useAppStore } from "./app-store";
import { useStartInstallPreview } from "./install-preview-sheet";

const SCENARIO_SWITCHES: { key: keyof MockScenario; label: string }[] = [
  { key: "emptyLibrary", label: "Empty library" },
  { key: "gameRunning", label: "Game running (writes locked)" },
  { key: "signedOut", label: "Signed out of mod.io" },
  { key: "offline", label: "Offline" },
  { key: "noApiKey", label: "No mod.io API key" },
];

export function MockStatesMenu() {
  const queryClient = useQueryClient();
  const game = useAppStore((store) => store.game);
  const setView = useAppStore((store) => store.setView);
  const setWizardOpen = useAppStore((store) => store.setWizardOpen);
  const startPreview = useStartInstallPreview();
  const [scenario, setScenario] = useState(mockScenario);

  const simulateDrop = (fileName: string) => {
    rememberMockGame(game);
    void startPreview([fileName]);
  };

  const refresh = () => {
    setScenario(mockScenario());
    void queryClient.invalidateQueries();
  };

  return (
    <div className="fixed right-4 bottom-12 z-40">
      <DropdownMenu>
        <DropdownMenuTrigger
          className={
            buttonVariants({ variant: "secondary", size: "sm" }) +
            " bg-bg-elevated shadow-overlay"
          }
        >
          <FlaskConicalIcon aria-hidden="true" className="size-4" />
          Mock-up states
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" side="top" className="w-80">
          <DropdownMenuGroup>
            <DropdownMenuLabel>
              Mock-up states (not in the app)
            </DropdownMenuLabel>
            {SCENARIO_SWITCHES.map((scenarioSwitch) => (
              <DropdownMenuCheckboxItem
                key={scenarioSwitch.key}
                checked={scenario[scenarioSwitch.key]}
                onCheckedChange={(checked) => {
                  setMockScenario({ [scenarioSwitch.key]: checked }, game);
                  refresh();
                }}
              >
                {scenarioSwitch.label}
              </DropdownMenuCheckboxItem>
            ))}
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuLabel>Flows</DropdownMenuLabel>
            <DropdownMenuItem
              onClick={() => simulateDrop(MOCK_DROP_SOURCES[game].pack)}
            >
              Drop: pack with update, downgrade and sub-mod
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => simulateDrop(MOCK_DROP_SOURCES[game].otherGame)}
            >
              Drop: mod for the other game
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => {
                restartSampleDownloads();
                setView("downloads");
                refresh();
              }}
            >
              Downloads: in progress, failed and done
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setWizardOpen(true)}>
              Open first-run wizard
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => {
                resetMockData();
                refresh();
              }}
            >
              Reset all mock data
            </DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
