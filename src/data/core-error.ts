import type { AppError, AppErrorCode } from "./types.ts";

/** A refusal from the core; an Error so `errorMessage()` and toasts read it like any other. */
export class CoreError extends Error implements AppError {
  readonly code: AppErrorCode;

  constructor(code: AppErrorCode, message: string) {
    super(message);
    this.name = "CoreError";
    this.code = code;
  }
}

const isAppError = (value: unknown): value is AppError =>
  typeof value === "object" &&
  value !== null &&
  "code" in value &&
  "message" in value &&
  typeof value.code === "string" &&
  typeof value.message === "string";

/** Tauri rejects with the serialized `AppError` object, not an Error; this restores one. */
export function toCoreError(rejection: unknown): Error {
  if (rejection instanceof Error) return rejection;
  if (isAppError(rejection))
    return new CoreError(rejection.code, rejection.message);
  return new CoreError(
    "io",
    typeof rejection === "string" ? rejection : "Something went wrong.",
  );
}
