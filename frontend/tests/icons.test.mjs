import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { iconMarkup } from "../src/icons.mjs";

test("every icon used by the page can be hydrated", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const names = [...html.matchAll(/data-icon="([^"]+)"/g)].map((match) => match[1]);

  for (const name of new Set(names)) {
    assert.doesNotThrow(() => iconMarkup(name), `Missing icon: ${name}`);
  }
});
