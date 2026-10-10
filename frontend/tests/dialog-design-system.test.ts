import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import BaseAlertDialog from "../src/components/base/BaseAlertDialog.vue";
import BaseDialog from "../src/components/base/BaseDialog.vue";

const wrappers: VueWrapper[] = [];
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  document.body.innerHTML = "";
});

describe("dialog design system", () => {
  it("defaults to a standard dialog and accepts the form and large sizes", async () => {
    const standard = mount(BaseDialog, { attachTo: document.body, props: { open: true, title: "标准" } });
    wrappers.push(standard);
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')?.getAttribute("data-dialog-size")).toBe("standard");
    expect(document.querySelector('[role="dialog"]')?.classList.contains("base-dialog--standard")).toBe(true);

    const form = mount(BaseDialog, { attachTo: document.body, props: { open: true, title: "表单", size: "form" } });
    wrappers.push(form);
    await flushPromises();
    const large = mount(BaseDialog, { attachTo: document.body, props: { open: true, title: "复杂内容", size: "large" } });
    wrappers.push(large);
    await flushPromises();
    expect(document.querySelector('[role="dialog"][data-dialog-size="form"]')?.classList.contains("base-dialog--form")).toBe(true);
    expect(document.querySelector('[role="dialog"][data-dialog-size="large"]')?.classList.contains("base-dialog--large")).toBe(true);
  });

  it("keeps AlertDialog semantics, defaults to compact, and focuses its cancel action", async () => {
    const alert = mount(BaseAlertDialog, {
      attachTo: document.body,
      props: { open: true, title: "確認" },
      slots: { footer: '<button type="button" data-dialog-cancel>取消</button><button type="button">確認</button>' },
    });
    wrappers.push(alert);
    await flushPromises();
    const dialog = document.querySelector<HTMLElement>('[role="alertdialog"]');
    expect(dialog?.getAttribute("data-dialog-size")).toBe("compact");
    expect(dialog?.classList.contains("base-dialog--compact")).toBe(true);
    expect(document.activeElement).toBe(dialog?.querySelector('[data-dialog-cancel]'));
  });
});
