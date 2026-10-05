import type { AccentColor } from "../../../contracts/v1/AccentColor";

export const accentPresetColors = {
  blue: "#3B82F6",
  indigo: "#6366F1",
  purple: "#A855F7",
  green: "#22C55E",
  orange: "#F97316",
  red: "#EF4444",
} as const satisfies Record<Exclude<AccentColor, "custom">, string>;

export function isValidAccentHex(value: string): boolean {
  return /^#[\da-f]{6}$/i.test(value);
}

export function resolveAccentColor(accent: AccentColor, custom: string | null): string {
  if (accent === "custom") return custom && isValidAccentHex(custom) ? custom : accentPresetColors.blue;
  return Object.hasOwn(accentPresetColors, accent) ? accentPresetColors[accent as keyof typeof accentPresetColors] : accentPresetColors.blue;
}

export function applyAccentColor(
  accent: AccentColor,
  custom: string | null,
  root: HTMLElement = document.documentElement,
) {
  root.style.setProperty("--color-accent-base", resolveAccentColor(accent, custom));
}
