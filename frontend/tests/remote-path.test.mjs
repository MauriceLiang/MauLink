import assert from "node:assert/strict";
import test from "node:test";
import { joinRemotePath, parentRemotePath } from "../src/remote-path.mjs";

test("parent remote path stays at root for a root or top-level entry", () => {
  assert.equal(parentRemotePath("/"), "/");
  assert.equal(parentRemotePath("/var"), "/");
  assert.equal(parentRemotePath("/var/log/"), "/var");
});

test("join remote path keeps absolute root and normalized parent separators", () => {
  assert.equal(joinRemotePath("/", "notes.txt"), "/notes.txt");
  assert.equal(joinRemotePath("/var/log/", "notes.txt"), "/var/log/notes.txt");
  assert.equal(joinRemotePath(".", "notes.txt"), "./notes.txt");
});
