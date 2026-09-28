const BYTE_UNITS = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

export function formatBytes(value) {
  if (value == null) return "—";
  let bytes;
  try { bytes = BigInt(value); } catch { return "—"; }
  if (bytes < 0n) return "—";
  if (bytes < 1024n) return `${bytes} B`;
  let divisor = 1024n;
  let unit = 1;
  while (bytes >= divisor * 1024n && unit < BYTE_UNITS.length - 1) {
    divisor *= 1024n;
    unit += 1;
  }
  const hundredths = Number((bytes * 100n + divisor / 2n) / divisor);
  const formatted = hundredths >= 10_000
    ? (hundredths / 100).toLocaleString(undefined, { maximumFractionDigits: 0 })
    : (hundredths / 100).toLocaleString(undefined, { maximumFractionDigits: 1 });
  return `${formatted} ${BYTE_UNITS[unit]}`;
}

export function formatRate(value) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) return "—";
  let scaled = value;
  let unit = 0;
  while (scaled >= 1024 && unit < BYTE_UNITS.length - 1) {
    scaled /= 1024;
    unit += 1;
  }
  const digits = scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2;
  return `${scaled.toLocaleString(undefined, { maximumFractionDigits: digits })} ${BYTE_UNITS[unit]}/s`;
}

export function formatPercent(value) {
  if (typeof value !== "number" || !Number.isFinite(value)) return "—";
  return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })}%`;
}

export function formatUptime(value) {
  if (value == null) return "—";
  let seconds;
  try { seconds = BigInt(value); } catch { return "—"; }
  if (seconds < 0n) return "—";
  const days = seconds / 86_400n;
  const hours = (seconds % 86_400n) / 3_600n;
  const minutes = (seconds % 3_600n) / 60n;
  return days > 0n ? `${days} 天 ${hours} 小时` : `${hours} 小时 ${minutes} 分钟`;
}

export function qualityLabel(status) {
  return ({
    ok: "正常",
    warmingUp: "正在采样",
    stale: "数据已过期",
    unsupported: "不支持",
    error: "暂不可用",
  })[status] ?? "等待数据";
}

export function sparklinePath(samples, width = 220, height = 42) {
  const values = (Array.isArray(samples) ? samples : [])
    .map((sample) => sample?.value)
    .filter((value) => typeof value === "number" && Number.isFinite(value));
  if (values.length === 0) return "";
  if (values.length === 1) return `M 0 ${height / 2} L ${width} ${height / 2}`;

  const minimum = Math.min(...values);
  const maximum = Math.max(...values);
  const range = maximum - minimum;
  const inset = 2;
  const points = values.map((value, index) => {
    const x = (index / (values.length - 1)) * width;
    const normalized = range === 0 ? 0.5 : (value - minimum) / range;
    const y = height - inset - normalized * (height - inset * 2);
    return `${index === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
  });
  return points.join(" ");
}
