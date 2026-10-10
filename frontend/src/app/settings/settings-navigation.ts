export type SettingsSection = 'general' | 'security' | 'appearance' | 'files' | 'network' | 'terminal' | 'language';

export type SettingsNavigationItem = {
  id: SettingsSection;
  label: 'general' | 'securityPrivacy' | 'appearance' | 'files' | 'network' | 'terminal' | 'language';
  icon: 'settings' | 'shield' | 'eye' | 'folder' | 'network' | 'terminal' | 'globe';
};

const items: SettingsNavigationItem[] = [
  { id: 'general', label: 'general', icon: 'settings' },
  { id: 'security', label: 'securityPrivacy', icon: 'shield' },
  { id: 'appearance', label: 'appearance', icon: 'eye' },
  { id: 'files', label: 'files', icon: 'folder' },
  { id: 'network', label: 'network', icon: 'network' },
  { id: 'terminal', label: 'terminal', icon: 'terminal' },
  { id: 'language', label: 'language', icon: 'globe' },
];

export function settingsNavigation(hasGeoIp: boolean) {
  return items.filter(item => item.id !== 'network' || hasGeoIp);
}
