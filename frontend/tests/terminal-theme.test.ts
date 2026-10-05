import { describe, expect, it } from 'vitest';
import { defaultSettings } from '../src/terminal/preferences';
import { resolveTerminalAppearance } from '../src/terminal/theme';

describe('terminal theme resolver', () => {
  it('follows an explicit app theme and the system theme only for System', () => {
    expect(resolveTerminalAppearance(defaultSettings, 'light', true).mode).toBe('light');
    expect(resolveTerminalAppearance(defaultSettings, 'dark', false).mode).toBe('dark');
    expect(resolveTerminalAppearance(defaultSettings, 'system', false).mode).toBe('light');
    expect(resolveTerminalAppearance(defaultSettings, 'system', true).mode).toBe('dark');
  });

  it('keeps explicit Light and Dark modes independent of the app theme', () => {
    expect(resolveTerminalAppearance({ ...defaultSettings, terminalThemeMode: 'light' }, 'dark', true).theme.background).toBe('#FFFFFF');
    expect(resolveTerminalAppearance({ ...defaultSettings, terminalThemeMode: 'dark' }, 'light', false).theme.background).toBe('#111318');
  });

  it('uses validated custom colors and derives an alpha selection color', () => {
    const appearance = resolveTerminalAppearance({
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
    const appearance = resolveTerminalAppearance({
      ...defaultSettings,
      terminalThemeMode: 'image',
      terminalCustomColors: { background: '#102030', foreground: '#E0E0E0', cursor: '#33AAFF', selection: '#7755CC' },
    }, 'light', false);
    expect(appearance.mode).toBe('image');
    expect(appearance.theme).toEqual({ background: 'transparent', foreground: '#E0E0E0', cursor: '#33AAFF', selectionBackground: 'rgba(119, 85, 204, 0.28)' });
    expect(appearance.transparent).toBe(true);
  });
});
