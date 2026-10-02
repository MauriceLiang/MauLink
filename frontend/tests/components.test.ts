import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import BaseButton from "../src/components/base/BaseButton.vue";
import BaseDialog from "../src/components/base/BaseDialog.vue";
import BaseInput from "../src/components/base/BaseInput.vue";
import FoundationsHarness from "../src/harness/FoundationsHarness.vue";

const wrappers: VueWrapper[] = [];
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  document.body.innerHTML = "";
});

describe("base keyboard interactions", () => {
  it("focuses and activates an enabled button", () => {
    const onClick = vi.fn();
    const wrapper = mount(BaseButton, { attachTo: document.body, props: { onClick }, slots: { default: "动作" } });
    wrappers.push(wrapper);
    const button = wrapper.get("button").element;
    button.focus();
    expect(document.activeElement).toBe(button);
    button.click();
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(button.disabled).toBe(false);
    expect(button.getAttribute("aria-busy")).toBe("false");
  });

  it("keeps an initially disabled button inactive and outside focus navigation", () => {
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();
    const onClick = vi.fn();
    const wrapper = mount(BaseButton, { attachTo: document.body, props: { disabled: true, onClick } });
    wrappers.push(wrapper);
    const button = wrapper.get("button").element;
    expect(button.disabled).toBe(true);
    expect(button.hasAttribute("disabled")).toBe(true);
    expect(button.getAttribute("aria-busy")).toBe("false");
    expect(button.classList.contains("base-button")).toBe(true);
    button.focus();
    expect(document.activeElement).toBe(opener);
    button.click();
    expect(onClick).not.toHaveBeenCalled();
  });

  it("blocks actions after a focused button becomes disabled or loading", async () => {
    const onClick = vi.fn();
    const wrapper = mount(BaseButton, { attachTo: document.body, props: { onClick } });
    wrappers.push(wrapper);
    const button = wrapper.get("button").element;
    button.focus();
    expect(document.activeElement).toBe(button);
    button.click();
    await wrapper.setProps({ disabled: true });
    expect(button.disabled).toBe(true);
    expect(button.hasAttribute("disabled")).toBe(true);
    button.click();
    expect(onClick).toHaveBeenCalledTimes(1);
    // Native focus removal on dynamic disable varies by DOM runtime; test the action contract.
    await wrapper.setProps({ disabled: false, loading: true });
    expect(button.disabled).toBe(true);
    expect(button.getAttribute("aria-busy")).toBe("true");
    button.click();
    expect(onClick).toHaveBeenCalledTimes(1);
    await wrapper.setProps({ loading: false });
    button.focus();
    expect(document.activeElement).toBe(button);
    button.click();
    expect(onClick).toHaveBeenCalledTimes(2);
  });

  function pressKey(element: HTMLElement, key: string, shiftKey = false) {
    const event = new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true });
    element.dispatchEvent(event);
    return event;
  }

  it("loops through mixed focusables in DOM order and restores trigger focus after Escape", async () => {
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();
    const wrapper = mount(BaseDialog, { attachTo: document.body, props: { open: true, title: "测试" },
      slots: {
        default: '<input aria-label="示例" /><button type="button" data-focus="middle">中间按钮</button>',
        footer: '<input data-focus="last" aria-label="末尾输入" /><button disabled tabindex="0">禁用按钮</button><input disabled tabindex="0" /><input tabindex="-1" /><input type="hidden" /><div hidden><button>隐藏按钮</button></div>',
      } });
    wrappers.push(wrapper);
    await flushPromises();
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!;
    const firstFocusable = dialog.querySelector<HTMLElement>('[aria-label="关闭对话框"]')!;
    const lastFocusable = dialog.querySelector<HTMLElement>('[data-focus="last"]')!;
    expect(document.activeElement).toBe(firstFocusable);
    expect(pressKey(firstFocusable, "Tab", true).defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(lastFocusable);
    expect(pressKey(lastFocusable, "Tab").defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(firstFocusable);
    const middleFocusable = dialog.querySelector<HTMLElement>('[data-focus="middle"]')!;
    middleFocusable.focus();
    expect(pressKey(middleFocusable, "Tab").defaultPrevented).toBe(false);
    pressKey(dialog, "Escape");
    expect(wrapper.emitted("close")).toHaveLength(1);
    await wrapper.setProps({ open: false });
    await flushPromises();
    expect(document.activeElement).toBe(opener);
    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });

  it("keeps the last DOM button in the loop when it follows an input", async () => {
    const wrapper = mount(BaseDialog, { attachTo: document.body, props: { open: true, title: "测试" },
      slots: { default: '<input aria-label="示例" /><button data-focus="last">最后一个</button>' } });
    wrappers.push(wrapper);
    await flushPromises();
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!;
    const firstFocusable = dialog.querySelector<HTMLElement>('[aria-label="关闭对话框"]')!;
    const lastFocusable = dialog.querySelector<HTMLElement>('[data-focus="last"]')!;
    pressKey(firstFocusable, "Tab", true);
    expect(document.activeElement).toBe(lastFocusable);
    pressKey(lastFocusable, "Tab");
    expect(document.activeElement).toBe(firstFocusable);
  });

  it("keeps ordinary tabindex in visual DOM order including custom focusables", async () => {
    const wrapper = mount(BaseDialog, { attachTo: document.body, props: { open: true, title: "测试" },
      slots: { default: '<div tabindex="0" data-focus="custom">自定义焦点</div><input data-focus="last" />' } });
    wrappers.push(wrapper);
    await flushPromises();
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!;
    const firstFocusable = dialog.querySelector<HTMLElement>('[aria-label="关闭对话框"]')!;
    const lastFocusable = dialog.querySelector<HTMLElement>('[data-focus="last"]')!;
    expect(document.activeElement).toBe(firstFocusable);
    pressKey(firstFocusable, "Tab", true);
    expect(document.activeElement).toBe(lastFocusable);
    pressKey(lastFocusable, "Tab");
    expect(document.activeElement).toBe(firstFocusable);
  });

  it("excludes the busy close button from focus and preserves the Escape rule", async () => {
    const wrapper = mount(BaseDialog, { attachTo: document.body, props: { open: true, busy: true, title: "处理中" },
      slots: { default: '<input data-focus="first" /><input data-focus="last" />' } });
    wrappers.push(wrapper);
    await flushPromises();
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!;
    const firstFocusable = dialog.querySelector<HTMLElement>('[data-focus="first"]')!;
    const lastFocusable = dialog.querySelector<HTMLElement>('[data-focus="last"]')!;
    expect(document.activeElement).toBe(firstFocusable);
    pressKey(firstFocusable, "Tab", true);
    expect(document.activeElement).toBe(lastFocusable);
    pressKey(lastFocusable, "Tab");
    expect(document.activeElement).toBe(firstFocusable);
    expect(pressKey(dialog, "Escape").defaultPrevented).toBe(true);
    expect(wrapper.emitted("close")).toBeUndefined();
    const closeButton = dialog.querySelector<HTMLButtonElement>('[aria-label="关闭对话框"]')!;
    expect(closeButton.disabled).toBe(true);
    closeButton.click();
    expect(wrapper.emitted("close")).toBeUndefined();
    await wrapper.setProps({ busy: false });
    pressKey(dialog, "Escape");
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("associates input labels and validation errors and emits user edits", async () => {
    const wrapper = mount(BaseInput, { props: { label: "名称", modelValue: "", error: "不能为空" } });
    wrappers.push(wrapper);
    const input = wrapper.get("input");
    expect(wrapper.get("label").attributes("for")).toBe(input.attributes("id"));
    expect(input.attributes("aria-describedby")).toBe(wrapper.get("p").attributes("id"));
    await input.setValue("fixture");
    expect(wrapper.emitted("update:modelValue")).toEqual([["fixture"]]);
  });

  it("loads and retries the browser harness through Mock IPC without Tauri", async () => {
    const wrapper = mount(FoundationsHarness, { attachTo: document.body });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.text()).toContain("浏览器 Mock IPC 已加载：0 台服务器");
    const findButton = (text: string) => wrapper.findAll("button").find(button => button.text() === text)!;
    await findButton("模拟 IPC 错误").trigger("click");
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain("连接超时");
    await findButton("重试").trigger("click");
    await flushPromises();
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(wrapper.text()).toContain("浏览器 Mock IPC 已加载：0 台服务器");
  });
});
