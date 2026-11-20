import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { errorMessage } from "./format";

/**
 * The desktop webview ignores `target="_blank"`, so links open in the user's browser.
 * Only https://mod.io links: the app's opener permission allows no other host.
 */
export function ExternalLink({
  href,
  children,
}: {
  href: string;
  children: ReactNode;
}) {
  return (
    <a
      className="text-accent underline underline-offset-4"
      href={href}
      target="_blank"
      rel="noreferrer"
      onClick={(event) => {
        if (!isTauri()) return;
        event.preventDefault();
        openUrl(href).catch((error: unknown) =>
          toast.error(errorMessage(error)),
        );
      }}
    >
      {children}
    </a>
  );
}
