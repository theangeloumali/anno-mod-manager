import { KeyRoundIcon, TrashIcon } from "lucide-react";
import { useState } from "react";
import { FIELD_CLASS } from "@/components/app/field-class";
import { errorMessage } from "@/components/app/format";
import { withToast } from "@/components/app/notify";
import { Button } from "@/components/ui/button";
import { useAccount, useAction } from "@/data/source";

const MODIO_ACCESS_URL = "https://mod.io/me/access";

/** Each user brings their own mod.io API key; the app never ships or shares one. */
export function ApiKeyPanel() {
  const { data: account } = useAccount();
  const [apiKey, setApiKey] = useState("");
  const [keyError, setKeyError] = useState<string | null>(null);
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
    <form
      className={box}
      onSubmit={async (event) => {
        event.preventDefault();
        setKeyError(null);
        try {
          await saveKey.mutateAsync([apiKey.trim()]);
          setApiKey("");
        } catch (error) {
          setKeyError(errorMessage(error));
        }
      }}
    >
      <label htmlFor="modio-api-key" className="text-ui font-semibold">
        Your mod.io API key
      </label>
      <p id="modio-api-key-hint" className="text-label text-copy-muted">
        Browsing mod.io needs your own key. Get it free at{" "}
        <a
          className="text-accent underline underline-offset-4"
          href={MODIO_ACCESS_URL}
          target="_blank"
          rel="noreferrer"
        >
          mod.io/me/access
        </a>{" "}
        under "API Access". It's stored in Windows Credential Manager, never in
        a file.
      </p>
      <div className="flex flex-col gap-2 sm:flex-row">
        <input
          id="modio-api-key"
          type="password"
          autoComplete="off"
          spellCheck={false}
          required
          value={apiKey}
          aria-invalid={keyError ? true : undefined}
          aria-describedby={
            keyError ? "modio-api-key-error" : "modio-api-key-hint"
          }
          onChange={(event) => setApiKey(event.target.value)}
          className={`${FIELD_CLASS} font-mono`}
          placeholder="32-character key"
        />
        <Button type="submit" variant="steel" disabled={saveKey.isPending}>
          Save key
        </Button>
      </div>
      {keyError && (
        <p
          id="modio-api-key-error"
          role="alert"
          className="text-ui text-danger"
        >
          {keyError}
        </p>
      )}
    </form>
  );
}
