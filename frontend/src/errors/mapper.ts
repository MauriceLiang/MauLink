import type { AppError } from "../../../contracts/v1/AppError";

function isAppError(error: unknown): error is AppError {
  if (!error || typeof error !== "object") return false;
  const value = error as Record<string, unknown>;
  return typeof value.code === "string"
    && typeof value.messageKey === "string"
    && typeof value.retryable === "boolean"
    && ["none", "retry", "reload", "reviewHostKey"].includes(String(value.action))
    && !!value.params && typeof value.params === "object" && !Array.isArray(value.params)
    && Object.values(value.params).every(param => typeof param === "string")
    && (value.stage === null || typeof value.stage === "string")
    && (value.requestId === null || typeof value.requestId === "string")
    && (value.details === null || typeof value.details === "string");
}

export function mapError(error: unknown, requestId: string | null = null): AppError {
  if (isAppError(error)) return { ...error, requestId: error.requestId ?? requestId };
  return {
    code: "INTERNAL",
    messageKey: "errors.unexpected",
    params: {},
    retryable: false,
    action: "none",
    stage: null,
    requestId,
    details: null,
  };
}
