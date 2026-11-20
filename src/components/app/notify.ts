// Created by The Angelo Umali.
import { toast } from "sonner";
import { errorMessage } from "./format";

/** `ok` rather than the value, because a void action's success is `undefined` too. */
export type ToastOutcome<TResult> =
  | { ok: true; value: TResult }
  | { ok: false };

/** Every write reports back: a success toast, or the core's own refusal text. */
export async function withToast<TResult>(
  work: Promise<TResult>,
  successMessage: string | ((result: TResult) => string),
  successAction?: { label: string; onClick: () => void },
): Promise<ToastOutcome<TResult>> {
  try {
    const result = await work;
    toast.success(
      typeof successMessage === "function"
        ? successMessage(result)
        : successMessage,
      successAction && { action: successAction },
    );
    return { ok: true, value: result };
  } catch (error) {
    toast.error(errorMessage(error));
    return { ok: false };
  }
}
