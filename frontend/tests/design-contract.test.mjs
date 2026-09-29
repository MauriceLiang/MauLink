import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("connection diagnostics stay collapsed until requested", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const details = html.match(/<details\s+class="connection-error-details"([^>]*)>/);

  assert.ok(details, "connection error details disclosure exists");
  assert.doesNotMatch(details[1], /\bopen\b/);
});
