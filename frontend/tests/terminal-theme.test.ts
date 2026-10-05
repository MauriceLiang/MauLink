import { describe, expect, it } from 'vitest';
import { defaultSettings } from '../src/terminal/preferences';
import { resolveTerminalTheme } from '../src/terminal/theme';
import { resolveTerminalAppearance } from '../src/terminal/appearance';
import type { ServerAppearance } from '../../contracts/v1/ServerAppearance';

describe('terminal theme resolver', () => {
  it('follows an explicit app theme and the system theme only for System', () => {
    expect(resolveTerminalTheme(defaultSettings, 'light', true).mode).toBe('light');
    expect(resolveTerminalTheme(defaultSettings, 'dark', false).mode).toBe('dark');
    expect(resolveTerminalTheme(defaultSettings, 'system', false).mode).toBe('light');
    expect(resolveTerminalTheme(defaultSettings, 'system', true).mode).toBe('dark');
  });

  it('keeps explicit Light and Dark modes independent of the app theme', () => {
    expect(resolveTerminalTheme({ ...defaultSettings, terminalThemeMode: 'light' }, 'dark', true).theme.background).toBe('#FFFFFF');
    expect(resolveTerminalTheme({ ...defaultSettings, terminalThemeMode: 'dark' }, 'light', false).theme.background).toBe('#111318');
  });

  it('uses validated custom colors and derives an alpha selection color', () => {
    const appearance = resolveTerminalTheme({
      ...defaultSettings,
      terminalThemeMode: 'customColor',
      terminalCustomColors: { background: '#102030', foreground: '#E0E0E0', cursor: '#33AAFF', selection: '#7755CC' },
    }, 'light', false);
    expect(appearance).toEqual({
      mode: 'customColor',
      theme: { background: '#102030', foreground: '#E0E0E0', cursor: '#33AAFF', selectionBackground: 'rgba(119, 85, 204, 0.28)' },
      transparent: false,
    });
  });

  it('resolves Image mode to a transparent xterm theme and keeps custom text colors', () => {
    const appearance = resolveTerminalTheme({
      ...defaultSettings,
      terminalThemeMode: 'image',
      terminalCustomColors: { background: '#102030', foreground: '#E0E0E0', cursor: '#33AAFF', selection: '#7755CC' },
    }, 'light', false);
    expect(appearance.mode).toBe('image');
    expect(appearance.theme).toEqual({ background: 'transparent', foreground: '#E0E0E0', cursor: '#33AAFF', selectionBackground: 'rgba(119, 85, 204, 0.28)' });
    expect(appearance.transparent).toBe(true);
  });
});

describe('per-server terminal appearance resolver', () => {
  const serverAppearance: ServerAppearance = {
    serverId: 'server-a', labelColor: '#D92D20', environment: 'production',
    terminalOverrideEnabled: true,
    terminalAppearance: {
      themeMode: 'dark',
      customColors: { background: '#120000', foreground: '#FEE4E2', cursor: '#D92D20', selection: '#F97066' },
      backgroundImage: { imageId: null, fit: 'cover', position: 'center', imageOpacity: 100, overlayKind: 'dark', overlayOpacity: 45, blurPx: 0 },
    },
    revision: 1, updatedAtMs: 1,
  };

  it('uses global settings unless this server has an enabled override', () => {
    expect(resolveTerminalAppearance('server-a', defaultSettings, { ...serverAppearance, terminalOverrideEnabled: false })).toBe(defaultSettings);
    expect(resolveTerminalAppearance('server-b', defaultSettings, serverAppearance)).toBe(defaultSettings);
    const effective = resolveTerminalAppearance('server-a', defaultSettings, serverAppearance);
    expect(effective.terminalThemeMode).toBe('dark');
    expect(effective.terminalCustomColors).toEqual(serverAppearance.terminalAppearance.customColors);
    expect(effective.terminalBackgroundImage).toEqual(serverAppearance.terminalAppearance.backgroundImage);
    expect(effective.terminalFontSize).toBe(defaultSettings.terminalFontSize);
  });
});
