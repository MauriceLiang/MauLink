export const geoIpMessages = {
  retry: ['重新读取状态', 'Reload status'],
  title: ['离线 IP 数据库', 'Offline IP databases'],
  intro: ['一键下载免费的位置与 ASN 数据库，无需账号。也可导入已有的 MMDB 文件；IP 查询在本机完成。', 'Download free location and ASN databases without an account, or import an MMDB file. IP lookups run locally.'],
  location: ['位置数据库', 'Location database'], asn: ['ASN 数据库', 'ASN database'],
  notConfigured: ['未配置', 'Not configured'], unavailable: ['文件不可用', 'File unavailable'], loading: ['正在读取数据库状态…', 'Loading database status…'],
  install: ['一键下载配置', 'Download and set up'], updateNow: ['立即检查更新', 'Check for updates'], import: ['导入 MMDB 文件', 'Import MMDB file'], delete: ['删除数据库', 'Delete databases'],
  frequency: ['自动更新频率', 'Automatic update frequency'], manual: ['关闭自动更新', 'Off'], daily: ['每天', 'Daily'], weekly: ['每周', 'Weekly'], monthly: ['每月（推荐）', 'Monthly (recommended)'],
  automaticNote: ['应用运行时按所选频率检查更新。DB-IP Lite 每月发布新版本；下载失败时继续使用已有数据库。', 'Checks for updates at this frequency while the app is running. DB-IP Lite releases monthly; existing databases remain usable if an update fails.'],
  localNote: ['导入文件后使用本地维护模式，不自动替换文件。一键下载配置后将启用所选更新频率。', 'Imported files are maintained manually. Download and set up to enable the selected update frequency.'],
  lastChecked: ['上次检查', 'Last checked'], updated: ['数据库已就绪。', 'Databases are ready.'], imported: ['数据库已导入，原文件保持不变。', 'Database imported. The original file is unchanged.'], deleted: ['应用中的数据库已删除。', 'App databases deleted.'], frequencySaved: ['更新频率已保存。', 'Update frequency saved.'],
  immediate: ['数据库操作与更新频率立即生效；删除仅清理应用保存的副本。', 'Database actions and update frequency apply immediately. Deletion removes only the app copies.'],
} as const;
