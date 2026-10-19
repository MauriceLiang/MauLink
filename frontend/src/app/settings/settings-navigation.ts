export type SettingsSection = 'general' | 'security' | 'appearance' | 'terminal';

export type SettingsNavigationItem = {
  id: SettingsSection;
  label: 'general' | 'securityPrivacy' | 'appearanceDisplay' | 'terminal';
};

const items: SettingsNavigationItem[] = [
  { id: 'general', label: 'general' },
  { id: 'security', label: 'securityPrivacy' },
  { id: 'appearance', label: 'appearanceDisplay' },
  { id: 'terminal', label: 'terminal' },
];

export function settingsNavigation() {
  return items;
}
