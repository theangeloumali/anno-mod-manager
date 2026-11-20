// Created by The Angelo Umali.
import { useQueryClient } from "@tanstack/react-query";
import {
  GlobeIcon,
  KeyRoundIcon,
  LogOutIcon,
  MonitorSmartphoneIcon,
  RefreshCwIcon,
  UserIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { FIELD_CLASS } from "@/components/app/field-class";
import { errorMessage } from "@/components/app/format";
import { withToast } from "@/components/app/notify";
import { Button } from "@/components/ui/button";
import { useAccount, useAction } from "@/data/source";
import type { AuthMethod, DeviceCode } from "@/data/types";

type AuthStep = "idle" | "terms" | "method" | "browser" | "device" | "token";

const APPROVAL_POLL_MS = 1000;

const METHODS: {
  method: AuthMethod;
  title: string;
  detail: string;
  icon: typeof GlobeIcon;
}[] = [
  {
    method: "browser",
    title: "Sign in with mod.io",
    detail: "Opens your browser. Nothing to copy.",
    icon: GlobeIcon,
  },
  {
    method: "device",
    title: "Use a device code",
    detail: "Enter a 5-character code at mod.io/connect on any device.",
    icon: MonitorSmartphoneIcon,
  },
  {
    method: "token",
    title: "Paste an access token",
    detail: "Create a read+write token at mod.io/me/access.",
    icon: KeyRoundIcon,
  },
];

const formatExpiry = (isoDate: string | null) =>
  isoDate
    ? new Date(isoDate).toLocaleDateString("en-GB", {
        day: "numeric",
        month: "short",
        year: "numeric",
      })
    : "never";

/** The whole mod.io sign-in flow: terms, then browser, device code or pasted token. */
export function AccountPanel() {
  const queryClient = useQueryClient();
  const { data: account, refetch } = useAccount();
  const [step, setStep] = useState<AuthStep>("idle");
  const [deviceCode, setDeviceCode] = useState<DeviceCode | null>(null);
  const [token, setToken] = useState("");
  const [tokenError, setTokenError] = useState<string | null>(null);
  const acceptTerms = useAction((source) => source.acceptTerms());
  const signIn = useAction((source, method: AuthMethod, pastedToken?: string) =>
    source.signIn(method, pastedToken),
  );
  const signOut = useAction((source) => source.signOut());
  const isWaiting = step === "browser" || step === "device";

  useEffect(() => {
    if (!isWaiting) return;
    const timer = window.setInterval(() => void refetch(), APPROVAL_POLL_MS);
    return () => window.clearInterval(timer);
  }, [isWaiting, refetch]);

  useEffect(() => {
    if (account?.signedIn && step !== "idle") {
      toast.success(`Signed in as ${account.username}`);
      setStep("idle");
      setDeviceCode(null);
      setToken("");
      void queryClient.invalidateQueries();
    }
  }, [account?.signedIn, account?.username, step, queryClient]);

  const chooseMethod = async (method: AuthMethod) => {
    if (method === "token") {
      setStep("token");
      return;
    }
    try {
      const code = await signIn.mutateAsync([method]);
      setDeviceCode(code);
      setStep(method);
    } catch (error) {
      toast.error(errorMessage(error));
    }
  };

  const box = "flex flex-col gap-3 rounded-field border border-line bg-bg p-4";

  if (account?.signedIn)
    return (
      <div className={`${box} sm:flex-row sm:items-center`}>
        <UserIcon aria-hidden="true" className="size-5 shrink-0 text-accent" />
        <div className="min-w-0 flex-1">
          <p className="text-ui font-semibold">{account.username}</p>
          <p className="text-label text-copy-muted">
            Signed in with mod.io · token in Windows Credential Manager ·
            expires {formatExpiry(account.expiresAt)}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            onClick={() =>
              void queryClient
                .invalidateQueries()
                .then(() => toast.success("Subscriptions synced from mod.io"))
            }
          >
            <RefreshCwIcon aria-hidden="true" />
            Sync subscriptions
          </Button>
          <Button
            size="sm"
            variant="quiet"
            onClick={() =>
              withToast(signOut.mutateAsync([]), "Signed out of mod.io")
            }
          >
            <LogOutIcon aria-hidden="true" />
            Sign out
          </Button>
        </div>
      </div>
    );

  if (step === "terms")
    return (
      <div className={box}>
        <h3 className="text-ui font-semibold">Before you sign in</h3>
        <p className="text-ui text-copy-muted">
          mod.io requires you to accept its{" "}
          <a
            className="text-accent underline underline-offset-4"
            href="https://mod.io/terms"
            target="_blank"
            rel="noreferrer"
          >
            Terms of Use
          </a>{" "}
          and{" "}
          <a
            className="text-accent underline underline-offset-4"
            href="https://mod.io/privacy"
            target="_blank"
            rel="noreferrer"
          >
            Privacy Policy
          </a>
          . If you decline, you can still browse and install as a guest.
        </p>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="steel"
            onClick={async () => {
              await withToast(acceptTerms.mutateAsync([]), "Terms accepted");
              setStep("method");
            }}
          >
            I agree
          </Button>
          <Button variant="quiet" onClick={() => setStep("idle")}>
            Decline
          </Button>
        </div>
      </div>
    );

  if (step === "method")
    return (
      <div className={box}>
        <h3 className="text-ui font-semibold">Choose how to sign in</h3>
        <ul className="grid gap-2">
          {METHODS.map(({ method, title, detail, icon: Icon }) => (
            <li key={method}>
              <button
                type="button"
                disabled={signIn.isPending}
                onClick={() => void chooseMethod(method)}
                className="flex w-full items-start gap-3 rounded-field border border-line-strong p-3 text-left transition-colors duration-(--dur-hover) hover:bg-surface"
              >
                <Icon
                  aria-hidden="true"
                  className="mt-0.5 size-5 shrink-0 text-accent"
                />
                <span>
                  <span className="block text-ui font-semibold">{title}</span>
                  <span className="block text-label text-copy-muted">
                    {detail}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>
        <Button
          variant="quiet"
          className="self-start"
          onClick={() => setStep("idle")}
        >
          Cancel
        </Button>
      </div>
    );

  if (step === "browser")
    return (
      <div className={box} aria-live="polite">
        <h3 className="text-ui font-semibold">
          Finish signing in in your browser
        </h3>
        <p className="text-ui text-copy-muted">
          We opened mod.io in your default browser. This updates by itself when
          it says you're done.
        </p>
        <Button
          variant="quiet"
          className="self-start"
          onClick={() => setStep("method")}
        >
          Use another method
        </Button>
      </div>
    );

  if (step === "device")
    return (
      <div className={box} aria-live="polite">
        <h3 className="text-ui font-semibold">
          Enter this code at{" "}
          <a
            className="text-accent underline underline-offset-4"
            href={deviceCode?.verifyUrl}
            target="_blank"
            rel="noreferrer"
          >
            mod.io/connect
          </a>
        </h3>
        <p
          className="font-mono text-h3 font-bold tracking-[0.4em] text-copy"
          aria-label={`Code ${deviceCode?.code.split("").join(" ")}`}
        >
          {deviceCode?.code}
        </p>
        <p className="text-ui text-copy-muted">
          Waiting for you to approve. The code expires in 10 minutes.
        </p>
        <Button
          variant="quiet"
          className="self-start"
          onClick={() => setStep("method")}
        >
          Back
        </Button>
      </div>
    );

  if (step === "token")
    return (
      <form
        className={box}
        onSubmit={async (event) => {
          event.preventDefault();
          setTokenError(null);
          try {
            await signIn.mutateAsync(["token", token]);
            await refetch();
          } catch (error) {
            setTokenError(errorMessage(error));
          }
        }}
      >
        <label htmlFor="modio-token" className="text-ui font-semibold">
          Access token
        </label>
        <p id="modio-token-hint" className="text-label text-copy-muted">
          Stored in Windows Credential Manager, never in a settings file.
        </p>
        <div className="flex flex-col gap-2 sm:flex-row">
          <input
            id="modio-token"
            type="password"
            autoComplete="off"
            required
            value={token}
            aria-invalid={tokenError ? true : undefined}
            aria-describedby={
              tokenError ? "modio-token-error" : "modio-token-hint"
            }
            onChange={(event) => setToken(event.target.value)}
            className={FIELD_CLASS}
            placeholder="Paste token"
          />
          <Button type="submit" variant="steel" disabled={signIn.isPending}>
            Connect
          </Button>
        </div>
        {tokenError && (
          <p
            id="modio-token-error"
            role="alert"
            className="text-ui text-danger"
          >
            {tokenError}
          </p>
        )}
        <Button
          type="button"
          variant="quiet"
          className="self-start"
          onClick={() => setStep("method")}
        >
          Back
        </Button>
      </form>
    );

  return (
    <div className={`${box} sm:flex-row sm:items-center`}>
      <UserIcon aria-hidden="true" className="size-5 shrink-0 text-copy-dim" />
      <div className="min-w-0 flex-1">
        <p className="text-ui font-semibold">Not signed in</p>
        <p className="text-label text-copy-muted">
          Browsing and installing work as a guest. Sign in to subscribe, rate
          and sync.
        </p>
      </div>
      <Button
        variant="steel"
        onClick={() => setStep(account?.termsAccepted ? "method" : "terms")}
      >
        Sign in to mod.io
      </Button>
    </div>
  );
}
