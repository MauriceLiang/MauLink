export type SettingsSection = 'general' | 'security' | 'appearance' | 'files' | 'terminal' | 'language';

export type SettingsNavigationItem = {
  id: SettingsSection;
  label: 'general' | 'securityPrivacy' | 'appearance' | 'files' | 'terminal' | 'language';
};

const items: SettingsNavigationItem[] = [
  { id: 'general', label: 'general' },
  { id: 'security', label: 'securityPrivacy' },
  { id: 'appearance', label: 'appearance' },
  { id: 'files', label: 'files' },
  { id: 'terminal', label: 'terminal' },
  { id: 'language', label: 'language' },
];

export function settingsNavigation() {
  return items;
}
