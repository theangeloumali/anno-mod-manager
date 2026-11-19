// Created by The Angelo Umali.
import { toast } from "sonner";
import { errorMessage } from "./format";

/** Every write reports back: a success toast, or the core's own refusal text. */
export async function withToast<TResult>(
  work: Promise<TResult>,
  successMessage: string | ((result: TResult) => string),
  successAction?: { label: string; onClick: () => void },
): Promise<TResult | undefined> {
  try {
    const result = await work;
    toast.success(
      typeof successMessage === "function"
        ? successMessage(result)
        : successMessage,
      successAction && { action: successAction },
    );
    return result;
  } catch (error) {
    toast.error(errorMessage(error));
    return undefined;
  }
}
