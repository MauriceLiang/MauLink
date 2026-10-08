import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerProfileDraft } from "../../../contracts/v1/ServerProfileDraft";
import type { CredentialUpdate } from "../../../contracts/v1/CredentialUpdate";
import type { SelectedLocalFile } from "../../../contracts/v1/SelectedLocalFile";
import type { ServerMessage } from "../i18n/servers";

export function newServerDraft(profile: ServerProfile | null = null, groupId: string | null = null): ServerProfileDraft {
  return {
    name: profile?.name ?? "", host: profile?.host ?? "", port: profile?.port ?? 22,
    username: profile?.username ?? "", authType: profile?.authType ?? "password",
    requireAuthentication: profile?.requireAuthentication ?? false, privateKeyToken: null,
    groupId: profile?.groupId ?? groupId, connectTimeoutMs: profile?.connectTimeoutMs ?? 15000,
    keepaliveIntervalSeconds: profile?.keepaliveIntervalSeconds ?? 30, jumpHost: profile?.jumpHost ?? "",
    jumpPort: profile?.jumpPort ?? 22, proxyType: profile?.proxyType ?? null,
    proxyHost: profile?.proxyHost ?? "", proxyPort: profile?.proxyPort ?? null,
  };
}

const portValid = (port: number | null) => Number.isInteger(port) && Number(port) >= 1 && Number(port) <= 65535;
const hostValid = (host: string) => !!host && Array.from(host).length <= 253 && !/[\s\u0000-\u001f@/\\]/u.test(host) && !host.includes("://");

export function validateServerDraft(draft: ServerProfileDraft, current: ServerProfile | null, key: SelectedLocalFile | null): ServerMessage | null {
  if (!hostValid(draft.host.trim())) return "hostRequired";
  if (!draft.username.trim() || Array.from(draft.username).length > 256 || /[\u0000\r\n]/u.test(draft.username)) return "userRequired";
  if (Array.from(draft.name ?? "").length > 128 || /[\u0000-\u001f\u007f]/u.test(draft.name ?? "")) return "nameInvalid";
  if (!portValid(draft.port) || !portValid(draft.jumpPort)) return "portInvalid";
  if (!Number.isInteger(draft.keepaliveIntervalSeconds) || draft.keepaliveIntervalSeconds < 5 || draft.keepaliveIntervalSeconds > 300) return "keepaliveInvalid";
  if (!Number.isInteger(draft.connectTimeoutMs) || draft.connectTimeoutMs < 1000 || draft.connectTimeoutMs > 120000) return "timeoutInvalid";
  const jump = draft.jumpHost?.trim();
  if (jump && (/[\s\u0000-\u001f]/u.test(jump) || jump.length > 512 || jump.split("@").length > 2 || !hostValid(jump.split("@").at(-1) ?? "") || jump.startsWith("@"))) return "jumpInvalid";
  if (draft.proxyType && (!hostValid(draft.proxyHost?.trim() ?? "") || !portValid(draft.proxyPort))) return "proxyInvalid";
  if (draft.authType === "privateKey") {
    if (!key && !(current?.authType === "privateKey" && current.hasPrivateKey)) return "keyRequired";
    if (key && (key.purpose !== "privateKey" || key.expiresAtMs <= Date.now())) return "keyExpired";
  }
  return null;
}

export function normalizeServerDraft(draft: ServerProfileDraft, key: SelectedLocalFile | null): ServerProfileDraft {
  return {
    ...draft, name: draft.name?.trim() || null, host: draft.host.trim(), username: draft.username.trim(),
    privateKeyToken: draft.authType === "privateKey" ? key?.token ?? null : null,
    jumpHost: draft.jumpHost?.trim() || null,
    proxyHost: draft.proxyType ? draft.proxyHost?.trim() || null : null,
    proxyPort: draft.proxyType ? draft.proxyPort : null,
  };
}

export function credentialValidation(current: ServerProfile | null, draft: ServerProfileDraft, credential: CredentialUpdate): ServerMessage | null {
  if (credential.mode === "replace" && !credential.secret) return "secretRequired";
  if (current?.hasSavedCredential && credential.mode === "keep"
    && (current.host !== draft.host || current.port !== draft.port || current.username !== draft.username
      || current.authType !== draft.authType || draft.privateKeyToken !== null)) return "identityChanged";
  return null;
}
