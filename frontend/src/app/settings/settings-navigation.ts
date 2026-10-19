export type SettingsSection = 'general' | 'security' | 'appearance' | 'terminal';

export type SettingsNavigationItem = {
  id: SettingsSection;
  label: 'general' | 'securityPrivacy' | 'appearanceDisplay' | 'terminal';
  icon: 'settings' | 'shield' | 'monitor' | 'terminal';
};

const items: SettingsNavigationItem[] = [
  { id: 'general', label: 'general', icon: 'settings' },
  { id: 'security', label: 'securityPrivacy', icon: 'shield' },
  { id: 'appearance', label: 'appearanceDisplay', icon: 'monitor' },
  { id: 'terminal', label: 'terminal', icon: 'terminal' },
];

export function settingsNavigation() {
  return items;
}
