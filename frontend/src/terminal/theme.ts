import type { AppSettings } from "../../../contracts/v1/AppSettings";
import type { TerminalThemeMode } from "../../../contracts/v1/TerminalThemeMode";
import type { Theme } from "../../../contracts/v1/Theme";
import { isValidAccentHex, resolveAccentColor } from "../theme/accent";

export interface TerminalAppearance {
  mode: Exclude<TerminalThemeMode, "followApp"> | "light" | "dark";
  theme: {
    background: string;
    foreground: string;
    cursor: string;
    selectionBackground: string;
  };
  transparent: boolean;
}

export function resolveTerminalAppearance(
  settings: AppSettings,
  appTheme: Theme,
  systemIsDark: boolean,
): TerminalAppearance {
  const mode = settings.terminalThemeMode === "followApp"
    ? appTheme === "system" ? (systemIsDark ? "dark" : "light") : appTheme
    : settings.terminalThemeMode;
  const accent = resolveAccentColor(settings.accentColor, settings.customAccentColor);

  if (mode === "customColor") {
    const colors = settings.terminalCustomColors;
    const background = safeHex(colors.background, "#111318");
    const foreground = safeHex(colors.foreground, "#EAECF0");
    const cursor = safeHex(colors.cursor, accent);
    const selection = safeHex(colors.selection, accent);
    return {
      mode,
      theme: { background, foreground, cursor, selectionBackground: withAlpha(selection, 0.28) },
      transparent: false,
    };
  }

  const light = mode === "light";
  const background = light ? "#FFFFFF" : "#111318";
  const foreground = light ? "#344054" : "#A7AFBC";
  return {
    mode,
    theme: { background, foreground, cursor: accent, selectionBackground: withAlpha(accent, 0.28) },
    transparent: false,
  };
}

function safeHex(value: string, fallback: string): string {
  return isValidAccentHex(value) ? value : fallback;
}

function withAlpha(hex: string, alpha: number): string {
  const safe = safeHex(hex, "#3B82F6").slice(1);
  const red = Number.parseInt(safe.slice(0, 2), 16);
  const green = Number.parseInt(safe.slice(2, 4), 16);
  const blue = Number.parseInt(safe.slice(4, 6), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}
