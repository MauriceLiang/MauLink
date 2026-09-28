import assert from "node:assert/strict";
import test from "node:test";

import { createTerminalOutputConsumer, decodeBase64, encodeBytesBase64, encodeTextBase64 } from "../src/terminal-codec.mjs";

test("text base64 codec round trips multi-byte input", () => {
  const value = "printf '你好 👋'\n";
  assert.deepEqual(decodeBase64(encodeTextBase64(value)), new TextEncoder().encode(value));
});

test("raw UTF-8 and VT bytes are ACKed in order only after the terminal parser consumes each chunk", async () => {
  const output = new TextEncoder().encode("你好\u001b[31m!");
  const source = [output.subarray(0, 2), output.subarray(2, 7), output.subarray(7)];
  const writes = [];
  const acknowledgements = [];
  const consume = createTerminalOutputConsumer(
    (bytes, callback) => writes.push({ bytes, callback }),
    async (chunk) => acknowledgements.push(chunk.seq),
  );
  const chunks = source.map((bytes, index) => ({
    seq: String(index + 1),
    byteLength: bytes.byteLength,
    dataBase64: encodeBytesBase64(bytes),
  }));

  const first = consume(chunks[0]);
  const second = consume(chunks[1]);
  const third = consume(chunks[2]);
  await Promise.resolve();
  assert.equal(writes.length, 1);
  assert.deepEqual(acknowledgements, []);

  writes[0].callback();
  await first;
  assert.deepEqual(acknowledgements, ["1"]);
  assert.equal(writes.length, 2);

  writes[1].callback();
  await second;
  assert.deepEqual(acknowledgements, ["1", "2"]);
  assert.equal(writes.length, 3);

  writes[2].callback();
  await third;
  assert.deepEqual(acknowledgements, ["1", "2", "3"]);
  assert.deepEqual(writes.map(({ bytes }) => [...bytes]), source.map((bytes) => [...bytes]));
});

test("terminal output is rejected without an ACK when its length contract is invalid", async () => {
  let writeCount = 0;
  let acknowledged = false;
  const consume = createTerminalOutputConsumer(
    (_bytes, callback) => { writeCount += 1; callback(); },
    async () => { acknowledged = true; },
  );

  await assert.rejects(consume({ seq: "1", byteLength: 2, dataBase64: encodeTextBase64("x") }), /长度校验/);
  assert.equal(writeCount, 0);
  assert.equal(acknowledged, false);
});

test("terminal output sequence gaps fail closed before bytes reach the parser", async () => {
  let writeCount = 0;
  let acknowledged = false;
  const consume = createTerminalOutputConsumer(
    (_bytes, callback) => { writeCount += 1; callback(); },
    async () => { acknowledged = true; },
  );

  await assert.rejects(consume({ seq: "2", byteLength: 1, dataBase64: encodeTextBase64("x") }), /序号不连续/);
  assert.equal(writeCount, 0);
  assert.equal(acknowledged, false);
});
