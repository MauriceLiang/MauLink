export const settingsMessages = {
  settings: ['设置', 'Settings'], sections: ['设置分类', 'Settings sections'], general: ['通用', 'General'], appearance: ['外观', 'Appearance'], terminal: ['终端', 'Terminal'], language: ['语言', 'Language'],
  confirmDisconnect: ['断开连接前确认', 'Confirm before disconnecting'], transferConfirmation: ['正在运行的传输仍需单独确认后才能取消。', 'Running transfers always require a separate confirmation before cancellation.'],
  theme: ['主题', 'Theme'], system: ['跟随系统', 'System'], light: ['浅色', 'Light'], dark: ['深色', 'Dark'], themeNote: ['保存后生效；系统主题随操作系统外观变化。', 'Applied on save. System theme follows operating system appearance.'], languageNote: ['保存后更新界面语言。服务器名称与远程内容保持原样。', 'Applied on save. Server names and remote content remain unchanged.'],
  terminalSettings: ['终端设置', 'Terminal settings'], terminalNote: ['字体、光标与滚动缓冲保存到 Rust SettingsService，并应用到活动终端。', 'Font, cursor and scrollback are saved by Rust SettingsService and applied to active terminals.'], reload: ['重新加载设置（放弃修改）', 'Reload settings (discard changes)'], cancel: ['取消', 'Cancel'], save: ['保存设置', 'Save settings'], saved: ['设置已保存。', 'Settings saved.'],
} as const;
