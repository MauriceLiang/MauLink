import test from "node:test";
import assert from "node:assert/strict";
import { copyTerminalSelection, isTerminalCopyOnSelectEnabled, setTerminalCopyOnSelectEnabled } from "../src/terminal-preferences.mjs";

function createStorage() {
  const values = new Map();
  return {
    getItem(key) {
      return values.get(key) ?? null;
    },
    setItem(key, value) {
      values.set(key, String(value));
    },
  };
}

test("terminal copy-on-select is disabled by default and persists the selected preference", () => {
  const storage = createStorage();

  assert.equal(isTerminalCopyOnSelectEnabled(storage), false);
  setTerminalCopyOnSelectEnabled(true, storage);
  assert.equal(isTerminalCopyOnSelectEnabled(storage), true);
  setTerminalCopyOnSelectEnabled(false, storage);
  assert.equal(isTerminalCopyOnSelectEnabled(storage), false);
});

test("terminal selection is copied only when enabled and non-empty", async () => {
  const copied = [];
  const clipboard = { writeText: async (text) => copied.push(text) };

  assert.equal(await copyTerminalSelection("secret", false, clipboard), false);
  assert.equal(await copyTerminalSelection("", true, clipboard), false);
  assert.equal(copied.length, 0);
  assert.equal(await copyTerminalSelection("selected output", true, clipboard), true);
  assert.deepEqual(copied, ["selected output"]);
});

test("enabled terminal copy reports an unavailable clipboard", async () => {
  await assert.rejects(copyTerminalSelection("selected output", true, undefined), /Clipboard API unavailable/);
});
