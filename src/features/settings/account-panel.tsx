// Created by The Angelo Umali.
import { useQueryClient } from "@tanstack/react-query";
import {
  KeyRoundIcon,
  LogOutIcon,
  MailIcon,
  RefreshCwIcon,
  UserIcon,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { ExternalLink } from "@/components/app/external-link";
import { withToast } from "@/components/app/notify";
import { Button } from "@/components/ui/button";
import { useAccount, useAction } from "@/data/source";
import type { AuthMethod } from "@/data/types";
import { SecretForm } from "./secret-form";

type AuthStep = "idle" | "terms" | "method" | "email" | "code" | "token";

const METHODS: {
  method: AuthMethod;
  title: string;
  detail: string;
  icon: typeof MailIcon;
}[] = [
  {
    method: "email",
    title: "Email me a code",
    detail: "mod.io emails you a 5-character code to type in here.",
    icon: MailIcon,
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

/** The whole mod.io sign-in flow: terms, then an emailed code or a pasted token. */
export function AccountPanel() {
  const queryClient = useQueryClient();
  const { data: account, refetch } = useAccount();
  const [step, setStep] = useState<AuthStep>("idle");
  const [email, setEmail] = useState("");
  // Bumped on every send and every Back, so only the latest open form's request counts.
  const codeRequest = useRef(0);
  const acceptTerms = useAction((source) => source.acceptTerms());
  const requestCode = useAction((source, address: string) =>
    source.requestSignInCode(address),
  );
  const signIn = useAction((source, method: AuthMethod, secret: string) =>
    source.signIn(method, secret),
  );
  const signOut = useAction((source) => source.signOut());

  useEffect(() => {
    if (account?.signedIn && step !== "idle") {
      toast.success(`Signed in as ${account.username}`);
      setStep("idle");
      void queryClient.invalidateQueries();
    }
  }, [account?.signedIn, account?.username, step, queryClient]);

  const sendCode = async (address: string) => {
    const request = ++codeRequest.current;
    await requestCode.mutateAsync([address]);
    if (request !== codeRequest.current) return;
    setEmail(address);
    setStep("code");
  };

  const finishSignIn = async (method: AuthMethod, secret: string) => {
    await signIn.mutateAsync([method, secret]);
    await refetch();
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
          <ExternalLink href="https://mod.io/terms">Terms of Use</ExternalLink>{" "}
          and{" "}
          <ExternalLink href="https://mod.io/privacy">
            Privacy Policy
          </ExternalLink>
          . If you decline, you can still browse and install as a guest.
        </p>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="steel"
            onClick={async () => {
              const accepted = await withToast(
                acceptTerms.mutateAsync([]),
                "Terms accepted",
              );
              if (accepted.ok) setStep("method");
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
                onClick={() => setStep(method)}
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

  if (step === "email")
    return (
      <SecretForm
        id="modio-email"
        label="Your mod.io email"
        hint="The address on your mod.io account. mod.io emails you a one-time code."
        type="email"
        placeholder="you@example.com"
        initialValue={email}
        submitLabel="Send code"
        isPending={requestCode.isPending}
        onSubmit={sendCode}
        onBack={() => {
          codeRequest.current += 1;
          setStep("method");
        }}
      />
    );

  if (step === "code")
    return (
      <SecretForm
        key="code"
        id="modio-code"
        label="Code from mod.io"
        hint={`Sent to ${email}. It can take a minute to arrive; check spam too.`}
        type="text"
        placeholder="5-character code"
        submitLabel="Sign in"
        isPending={signIn.isPending}
        onSubmit={(code) => finishSignIn("email", code)}
        onBack={() => setStep("email")}
        extra={
          <Button
            type="button"
            variant="quiet"
            // A new code can void the one being exchanged, so not mid sign-in.
            disabled={requestCode.isPending || signIn.isPending}
            onClick={() =>
              withToast(
                requestCode.mutateAsync([email]),
                `Sent a new code to ${email}`,
              )
            }
          >
            Send a new code
          </Button>
        }
      />
    );

  if (step === "token")
    return (
      <SecretForm
        id="modio-token"
        label="Access token"
        hint={
          <>
            Create a read+write token at{" "}
            <ExternalLink href="https://mod.io/me/access">
              mod.io/me/access
            </ExternalLink>
            . Stored in Windows Credential Manager, never in a settings file.
          </>
        }
        type="password"
        placeholder="Paste token"
        submitLabel="Connect"
        isPending={signIn.isPending}
        onSubmit={(token) => finishSignIn("token", token)}
        onBack={() => setStep("method")}
      />
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
