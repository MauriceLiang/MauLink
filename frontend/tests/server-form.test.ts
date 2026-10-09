import { describe, expect, it } from "vitest";
import { shellServers } from "../src/harness/shell-fixtures";
import {
  cancelReplace,
  credentialValidation,
  enterReplace,
  initialCredentialEditAction,
  markPendingClear,
  newServerDraft,
  resolveCredentialUpdate,
  undoPendingClear,
} from "../src/dialogs/server-form";

const savedProfile = { ...shellServers[0]!, hasSavedCredential: true };

describe("credential edit actions", () => {
  it("starts new servers in edit mode and existing profiles without changing credentials", () => {
    expect(initialCredentialEditAction(true)).toBe("editing");
    expect(initialCredentialEditAction(false)).toBe("unchanged");
  });

  it("enters and cancels replacement without carrying a draft secret forward", () => {
    expect(enterReplace()).toBe("editing");
    expect(cancelReplace("editing")).toBe("unchanged");
    expect(cancelReplace("pendingClear")).toBe("pendingClear");
  });

  it("marks removal for confirmation and allows undo before the overall save", () => {
    const pending = markPendingClear("unchanged");
    expect(pending).toBe("pendingClear");
    expect(undoPendingClear(pending)).toBe("unchanged");
    expect(markPendingClear("editing")).toBe("editing");
  });

  it("resolves unchanged, replacement, empty new credential, and removal actions", () => {
    expect(resolveCredentialUpdate("unchanged", "ignored", true)).toEqual({ mode: "keep" });
    expect(resolveCredentialUpdate("unchanged", "ignored", false)).toEqual({ mode: "keep" });
    expect(resolveCredentialUpdate("editing", "new-secret", true)).toEqual({ mode: "replace", secret: "new-secret" });
    expect(resolveCredentialUpdate("editing", "", false)).toEqual({ mode: "clear" });
    expect(resolveCredentialUpdate("editing", "", true)).toEqual({ mode: "replace", secret: "" });
    expect(resolveCredentialUpdate("pendingClear", "ignored", true)).toEqual({ mode: "clear" });
  });

  it("keeps identity validation when saved credentials are left unchanged", () => {
    const draft = { ...newServerDraft(savedProfile), host: "changed.example.com" };
    expect(credentialValidation(savedProfile, draft, resolveCredentialUpdate("unchanged", "", true))).toBe("identityChanged");
    expect(credentialValidation(savedProfile, draft, resolveCredentialUpdate("pendingClear", "", true))).toBeNull();
    expect(credentialValidation(savedProfile, draft, resolveCredentialUpdate("editing", "", true))).toBe("secretRequired");
  });
});
