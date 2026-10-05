import type { AppSettings } from "../../../contracts/v1/AppSettings";
import type { ServerAppearance } from "../../../contracts/v1/ServerAppearance";

export function resolveTerminalAppearance(
  serverId: string,
  globalSettings: AppSettings,
  serverAppearance?: ServerAppearance | null,
): AppSettings {
  if (!serverAppearance?.terminalOverrideEnabled || serverAppearance.serverId !== serverId) {
    return globalSettings;
  }
  return {
    ...globalSettings,
    terminalThemeMode: serverAppearance.terminalAppearance.themeMode,
    terminalCustomColors: serverAppearance.terminalAppearance.customColors,
    terminalBackgroundImage: serverAppearance.terminalAppearance.backgroundImage,
  };
}
