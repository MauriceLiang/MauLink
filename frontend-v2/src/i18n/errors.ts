export const errorMessages: Record<string, { "zh-CN": string; en: string }> = {
  "errors.unexpected": { "zh-CN": "操作失败，请稍后重试。", en: "The operation failed. Please try again later." },
  "errors.connectionTimeout": { "zh-CN": "连接超时，请检查服务器地址和网络。", en: "Connection timed out. Check the server address and network." },
  "errors.hostKeyChanged": { "zh-CN": "服务器身份与已保存的指纹不一致，连接已被阻止。", en: "The server identity has changed. The connection was blocked." },
  "errors.authenticationFailed": { "zh-CN": "认证失败，请检查用户名或凭据。", en: "Authentication failed. Check your username or credentials." },
  "errors.serverNameInvalid": { "zh-CN": "服务器名称不符合要求。", en: "The server name is invalid." },
  "errors.serverInUse": { "zh-CN": "服务器正在使用中，无法修改连接信息。", en: "This server is in use. Connection details cannot be changed." },
  "errors.ipcVersionUnsupported": { "zh-CN": "接口版本 {requestedVersion} 不兼容，请求版本应为 {supportedVersion}。", en: "API version {requestedVersion} is unsupported. Expected {supportedVersion}." },
  "errors.pathNotFound": { "zh-CN": "远程路径不存在，请刷新目录。", en: "The remote path was not found. Refresh the directory." },
  "errors.permissionDenied": { "zh-CN": "服务器拒绝了文件操作，请检查权限。", en: "The server denied the file operation. Check permissions." },
  "errors.activeTransfersRequireConfirmation": { "zh-CN": "此连接仍有 {count} 项传输任务，请先确认是否取消任务。", en: "This connection has {count} active transfers. Confirm whether to cancel them first." },
};

export const stageMessages: Record<string, { "zh-CN": string; en: string }> = {
  connection: { "zh-CN": "连接", en: "Connection" },
  authentication: { "zh-CN": "认证", en: "Authentication" },
  hostKey: { "zh-CN": "服务器身份验证", en: "Server identity verification" },
  terminal: { "zh-CN": "终端", en: "Terminal" },
  sftp: { "zh-CN": "文件操作", en: "File operation" },
  monitor: { "zh-CN": "监控", en: "Monitor" },
};
