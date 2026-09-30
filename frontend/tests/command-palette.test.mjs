import test from "node:test";
import assert from "node:assert/strict";
import { filterPaletteCommands, movePaletteSelection } from "../src/command-palette.mjs";

const commands = [
  { label: "Web 01", meta: "root@192.168.1.20", keywords: ["web-01", "server"] },
  { label: "打开监控", keywords: ["monitor"], disabled: true },
  { label: "添加新的服务器", keywords: ["add server"] },
];

test("filters palette commands by localized labels, metadata, and aliases", () => {
  assert.deepEqual(filterPaletteCommands(commands, "监控"), [commands[1]]);
  assert.deepEqual(filterPaletteCommands(commands, "192.168"), [commands[0]]);
  assert.deepEqual(filterPaletteCommands(commands, "add server"), [commands[2]]);
  assert.deepEqual(filterPaletteCommands(commands, "no match"), []);
  assert.deepEqual(filterPaletteCommands(commands, ""), commands);
});

test("moves the palette selection, wraps, and skips unavailable commands", () => {
  assert.equal(movePaletteSelection(commands, 0, 1), 2);
  assert.equal(movePaletteSelection(commands, 2, 1), 0);
  assert.equal(movePaletteSelection(commands, 0, -1), 2);
  assert.equal(movePaletteSelection([{ disabled: true }], 0, 1), -1);
  assert.equal(movePaletteSelection([], 0, 1), -1);
});
