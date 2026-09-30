import type { AppError } from "../../../contracts/v1/AppError";
import type { Language } from "../../../contracts/v1/Language";
import { errorMessages, stageMessages } from "../i18n/errors";

export function presentError(error: AppError, language: Language = "zh-CN") {
  const key = error.code === "HOST_KEY_CHANGED" ? "errors.hostKeyChanged" : error.messageKey;
  const template = (errorMessages[key] ?? errorMessages["errors.unexpected"]!)[language];
  return {
    message: template.replace(/\{(\w+)\}/g, (_, name: string) => error.params[name] ?? "—"),
    retryable: error.retryable && error.code !== "HOST_KEY_CHANGED",
    action: error.action,
    stage: error.stage ? stageMessages[error.stage]?.[language] ?? null : null,
    requestId: error.requestId,
  };
}
