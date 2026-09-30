import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("connection diagnostics stay collapsed until requested", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const details = html.match(/<details\s+class="connection-error-details"([^>]*)>/);

  assert.ok(details, "connection error details disclosure exists");
  assert.doesNotMatch(details[1], /\bopen\b/);
});

test("terminal copy-on-select is an explicit opt-in setting", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const setting = html.match(/<input id="setting-terminal-copy-on-select"[^>]*>/)?.[0];

  assert.ok(setting, "terminal copy-on-select setting exists");
  assert.match(setting, /type="checkbox"/);
  assert.doesNotMatch(setting, /\bchecked\b/);
});

test("terminal settings expose prototype-sized presets and retain advanced xterm options", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const terminalSettings = html.match(/<section class="settings-section" id="settings-section-terminal"[\s\S]*?<\/section>/)?.[0];

  assert.ok(terminalSettings, "terminal settings section exists");
  for (const size of [12, 13, 14, 15]) assert.match(terminalSettings, new RegExp(`data-terminal-font-size="${size}"`));
  assert.match(terminalSettings, /data-terminal-cursor="block"/);
  assert.match(terminalSettings, /data-terminal-cursor="bar"/);
  assert.match(terminalSettings, /data-terminal-cursor="underline"/);
  assert.match(terminalSettings, /<details class="terminal-advanced-settings">/);
  assert.match(terminalSettings, /id="setting-terminal-font"/);
  assert.match(terminalSettings, /id="setting-terminal-scrollback"/);
});
