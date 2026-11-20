import { KeyRoundIcon, TrashIcon } from "lucide-react";
import { ExternalLink } from "@/components/app/external-link";
import { withToast } from "@/components/app/notify";
import { Button } from "@/components/ui/button";
import { useAccount, useAction } from "@/data/source";
import { SecretForm } from "./secret-form";

const MODIO_ACCESS_URL = "https://mod.io/me/access";

/** Each user brings their own mod.io API key; the app never ships or shares one. */
export function ApiKeyPanel() {
  const { data: account } = useAccount();
  const saveKey = useAction((source, key: string) => source.setApiKey(key));
  const clearKey = useAction((source) => source.clearApiKey());
  const box = "flex flex-col gap-3 rounded-field border border-line bg-bg p-4";

  if (account?.hasApiKey)
    return (
      <div className={`${box} sm:flex-row sm:items-center`}>
        <KeyRoundIcon
          aria-hidden="true"
          className="size-5 shrink-0 text-accent"
        />
        <div className="min-w-0 flex-1">
          <p className="text-ui font-semibold">Your mod.io API key is saved</p>
          <p className="text-label text-copy-muted">
            Stored in Windows Credential Manager on this PC only.
          </p>
        </div>
        <Button
          size="sm"
          variant="quiet"
          onClick={() =>
            withToast(clearKey.mutateAsync([]), "mod.io API key removed")
          }
        >
          <TrashIcon aria-hidden="true" />
          Remove key
        </Button>
      </div>
    );

  return (
    <SecretForm
      id="modio-api-key"
      label="Your mod.io API key"
      hint={
        <>
          Browsing mod.io needs your own key. Get it free at{" "}
          <ExternalLink href={MODIO_ACCESS_URL}>mod.io/me/access</ExternalLink>{" "}
          under "API Access". It's stored in Windows Credential Manager, never
          in a file.
        </>
      }
      type="password"
      fieldClassName="font-mono"
      placeholder="32-character key"
      submitLabel="Save key"
      isPending={saveKey.isPending}
      onSubmit={(key) => saveKey.mutateAsync([key])}
    />
  );
}
