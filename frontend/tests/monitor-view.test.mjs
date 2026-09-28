import test from "node:test";
import assert from "node:assert/strict";

import { formatBytes, formatPercent, formatRate, formatUptime, qualityLabel, sparklinePath } from "../src/monitor-view.mjs";

test("formats byte counts and rates from decimal wire values", () => {
  assert.equal(formatBytes("0"), "0 B");
  assert.equal(formatBytes("1073741824"), "1 GiB");
  assert.equal(formatBytes("18446744073709551615"), "16 EiB");
  assert.equal(formatRate(1536), "1.5 KiB/s");
  assert.equal(formatRate(null), "—");
});

test("formats percentages, uptime, and missing values without inventing zero", () => {
  assert.equal(formatPercent(37.25), "37.3%");
  assert.equal(formatPercent(null), "—");
  assert.equal(formatUptime("90061"), "1 天 1 小时");
  assert.equal(formatUptime(null), "—");
  assert.equal(qualityLabel("warmingUp"), "正在采样");
  assert.equal(qualityLabel("unsupported"), "不支持");
});

test("creates bounded sparkline paths for empty, single, constant, and varying data", () => {
  assert.equal(sparklinePath([]), "");
  assert.equal(sparklinePath([{ value: 1 }]), "M 0 21 L 220 21");
  assert.equal(sparklinePath([{ value: 4 }, { value: 4 }]), "M 0.0 21.0 L 220.0 21.0");
  assert.equal(sparklinePath([{ value: 0 }, { value: 10 }]), "M 0.0 40.0 L 220.0 2.0");
  assert.equal(sparklinePath([{ value: NaN }, { value: Infinity }]), "");
});
