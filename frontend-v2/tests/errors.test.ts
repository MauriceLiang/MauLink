import { describe, expect, it } from "vitest";
import { mapError } from "../src/errors/mapper";
import { presentError } from "../src/errors/presenter";
import { timeoutError } from "../src/harness/fixtures";

describe("safe error presentation", () => {
  it("maps keys, params, retryability and stage in both locales", () => {
    const error = { ...timeoutError, messageKey: "errors.activeTransfersRequireConfirmation", params: { count: "3" } };
    expect(presentError(error)).toMatchObject({ message: expect.stringContaining("3"), retryable: true, stage: "连接" });
    expect(presentError(error, "en")).toMatchObject({ message: expect.stringContaining("3 active transfers"), stage: "Connection" });
  });
  it("never exposes raw exception text, debug details, or unknown message keys", () => {
    const mapped = mapError(new Error("password=must-not-be-shown"));
    expect(mapped.details).toBeNull();
    const presentation = presentError({ ...mapped, messageKey: "Rust debug unknown", details: "private path" });
    expect(presentation.message).toBe("操作失败，请稍后重试。");
    expect(JSON.stringify(presentation)).not.toContain("private path");
  });
  it("keeps host-key changes blocked even if an error claims to be retryable", () => {
    expect(presentError({ ...timeoutError, code: "HOST_KEY_CHANGED", action: "reviewHostKey" }))
      .toMatchObject({ message: expect.stringContaining("连接已被阻止"), retryable: false, action: "reviewHostKey" });
  });
});
