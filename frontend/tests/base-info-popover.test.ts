import { afterEach, expect, it } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import BaseInfoPopover from "../src/components/base/BaseInfoPopover.vue";

const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ""; });

it("opens an accessible click popover and closes it with Escape", async () => {
  const wrapper = mount(BaseInfoPopover, {
    attachTo: document.body,
    props: { title: "说明标题", description: "说明内容", triggerLabel: "说明" },
  });
  wrappers.push(wrapper);

  const trigger = wrapper.get("button");
  expect(trigger.attributes("type")).toBe("button");
  expect(trigger.attributes("aria-label")).toBe("说明");
  await trigger.trigger("click");
  await flushPromises();
  expect(document.querySelector(".base-popover-content")?.textContent).toContain("说明内容");

  await document.querySelector(".base-popover-content")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await flushPromises();
  expect(document.querySelector(".base-popover-content")).toBeNull();
});
