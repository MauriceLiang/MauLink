import type { ConnectionState } from "../../../contracts/v1/ConnectionState";
import type { Language } from "../../../contracts/v1/Language";
const states: Record<ConnectionState, { 'zh-CN': string; en: string }> = {
  created: { 'zh-CN': '准备连接', en: 'Preparing' }, resolving: { 'zh-CN': '解析主机', en: 'Resolving' }, connecting: { 'zh-CN': '正在连接', en: 'Connecting' }, verifyingHostKey: { 'zh-CN': '验证服务器', en: 'Verifying identity' }, awaitingHostTrust: { 'zh-CN': '等待身份确认', en: 'Awaiting host trust' }, awaitingCredentials: { 'zh-CN': '等待凭据', en: 'Awaiting credentials' }, authenticating: { 'zh-CN': '正在认证', en: 'Authenticating' }, ready: { 'zh-CN': '已连接', en: 'Connected' }, disconnecting: { 'zh-CN': '正在断开', en: 'Disconnecting' }, closed: { 'zh-CN': '已断开', en: 'Disconnected' }, failed: { 'zh-CN': '连接失败', en: 'Connection failed' }, cancelled: { 'zh-CN': '已取消', en: 'Cancelled' },
};
export const connectionStateText = (state: ConnectionState, language: Language = 'zh-CN') => states[state][language];
