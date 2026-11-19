// Created by The Angelo Umali.
import { MenuIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useAppStore } from "./app-store";
import { GameSwitcher } from "./game-switcher";
import { LaunchButton } from "./launch-button";
import { SidebarNav } from "./sidebar-nav";
import { StatusBar } from "./status-bar";

/** Desktop: header, sidebar, scrolling main, status bar. Under md the sidebar moves into a sheet. */
export function Shell({ children }: { children: ReactNode }) {
  const isMobileNavOpen = useAppStore((store) => store.isMobileNavOpen);
  const setMobileNavOpen = useAppStore((store) => store.setMobileNavOpen);

  return (
    <div className="grid h-dvh grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden md:grid-cols-[15rem_minmax(0,1fr)]">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded-control focus:bg-accent focus:px-4 focus:py-2 focus:text-on-accent"
      >
        Skip to content
      </a>
      <header className="col-span-full flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line bg-bg-elevated px-4 py-2.5">
        <Button
          variant="quiet"
          size="icon"
          className="-ml-2 md:hidden"
          aria-label="Open navigation"
          onClick={() => setMobileNavOpen(true)}
        >
          <MenuIcon aria-hidden="true" />
        </Button>
        <p className="flex items-baseline gap-2">
          <span className="font-display text-panel-title font-bold tracking-tight">
            Atelier
          </span>
          <span className="hidden text-label text-copy-dim sm:inline">
            Anno Mod Manager
          </span>
        </p>
        <div className="order-last w-full md:order-none md:ml-6 md:w-auto">
          <GameSwitcher />
        </div>
        <div className="ml-auto">
          <LaunchButton />
        </div>
      </header>

      <aside className="hidden overflow-y-auto border-r border-line bg-bg-elevated md:block">
        <SidebarNav />
      </aside>

      <main
        id="main"
        tabIndex={-1}
        aria-labelledby="page-title"
        className="min-w-0 overflow-x-hidden overflow-y-auto px-4 py-5 outline-none md:px-8 md:py-7"
      >
        {children}
      </main>

      <div className="col-span-full">
        <StatusBar />
      </div>

      <Sheet open={isMobileNavOpen} onOpenChange={setMobileNavOpen}>
        <SheetContent side="left" className="w-72 gap-0 bg-bg-elevated p-0">
          <SheetHeader className="border-b border-line">
            <SheetTitle className="font-display text-panel-title font-bold">
              Atelier
            </SheetTitle>
          </SheetHeader>
          <SidebarNav />
        </SheetContent>
      </Sheet>
    </div>
  );
}
