export type SettingsSection = 'general' | 'security' | 'appearance' | 'files' | 'network' | 'terminal' | 'language';

export type SettingsNavigationItem = {
  id: SettingsSection;
  label: 'general' | 'securityPrivacy' | 'appearance' | 'files' | 'network' | 'terminal' | 'language';
};

const items: SettingsNavigationItem[] = [
  { id: 'general', label: 'general' },
  { id: 'security', label: 'securityPrivacy' },
  { id: 'appearance', label: 'appearance' },
  { id: 'files', label: 'files' },
  { id: 'network', label: 'network' },
  { id: 'terminal', label: 'terminal' },
  { id: 'language', label: 'language' },
];

export function settingsNavigation(hasGeoIp: boolean) {
  return items.filter(item => item.id !== 'network' || hasGeoIp);
}
