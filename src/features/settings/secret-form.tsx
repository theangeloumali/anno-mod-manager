import { useState, type ReactNode } from "react";
import { FIELD_CLASS } from "@/components/app/field-class";
import { errorMessage } from "@/components/app/format";
import { Button } from "@/components/ui/button";

/** One field, one action, an inline error and a way back: each sign-in step. */
export function SecretForm({
  id,
  label,
  hint,
  type,
  placeholder,
  submitLabel,
  initialValue = "",
  fieldClassName = "",
  isPending,
  onSubmit,
  onBack,
  extra,
}: {
  id: string;
  label: string;
  hint: ReactNode;
  type: "email" | "text" | "password";
  /** Extra classes for the field, e.g. a monospace key. */
  fieldClassName?: string;
  placeholder: string;
  /** What the field starts with, e.g. the address when coming back to fix it. */
  initialValue?: string;
  submitLabel: string;
  isPending: boolean;
  onSubmit: (value: string) => Promise<void>;
  onBack?: () => void;
  extra?: ReactNode;
}) {
  const [value, setValue] = useState(initialValue);
  const [error, setError] = useState<string | null>(null);
  const hintId = `${id}-hint`;
  const errorId = `${id}-error`;

  return (
    <form
      className="flex flex-col gap-3 rounded-field border border-line bg-bg p-4"
      onSubmit={async (event) => {
        event.preventDefault();
        setError(null);
        try {
          await onSubmit(value.trim());
        } catch (caught) {
          setError(errorMessage(caught));
        }
      }}
    >
      <label htmlFor={id} className="text-ui font-semibold">
        {label}
      </label>
      <p id={hintId} className="text-label text-copy-muted">
        {hint}
      </p>
      <div className="flex flex-col gap-2 sm:flex-row">
        <input
          id={id}
          type={type}
          autoComplete={type === "email" ? "email" : "off"}
          spellCheck={false}
          required
          value={value}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : hintId}
          onChange={(event) => setValue(event.target.value)}
          className={`${FIELD_CLASS} ${fieldClassName}`}
          placeholder={placeholder}
        />
        <Button type="submit" variant="steel" disabled={isPending}>
          {submitLabel}
        </Button>
      </div>
      {error && (
        <p id={errorId} role="alert" className="text-ui text-danger">
          {error}
        </p>
      )}
      {(onBack || extra) && (
        <div className="flex flex-wrap gap-2">
          {onBack && (
            <Button type="button" variant="quiet" onClick={onBack}>
              Back
            </Button>
          )}
          {extra}
        </div>
      )}
    </form>
  );
}
