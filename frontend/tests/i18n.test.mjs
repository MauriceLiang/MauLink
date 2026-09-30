import test from "node:test";
import assert from "node:assert/strict";
import { getLocale, setLocale, translateText } from "../src/i18n.mjs";

test("translates interface labels while leaving user supplied values intact", () => {
  setLocale("en");
  assert.equal(translateText("添加服务器"), "Add Server");
  assert.equal(translateText("还没有服务器"), "No servers yet");
  assert.equal(translateText("连接你的第一台服务器。\n在一个工作区中使用终端、管理文件并查看服务器状态。"), "Connect your first server.\nUse the terminal, manage files and check server status in one workspace.");
  assert.equal(translateText("了解 MauLink"), "Learn about MauLink");
  assert.equal(translateText("更多操作"), "More actions");
  assert.equal(translateText("服务器操作"), "Server actions");
  assert.equal(translateText("1 个终端"), "1 terminal");
  assert.equal(translateText("2 个终端"), "2 terminals");
  assert.equal(translateText("选中即复制"), "Copy on select");
  assert.equal(translateText("自动将终端中选中的内容复制到剪贴板。"), "Automatically copy selected terminal text to the clipboard.");
  assert.equal(translateText("无法访问剪贴板，请使用键盘快捷键复制终端内容。"), "Could not access the clipboard. Use the keyboard shortcut to copy terminal text.");
  assert.equal(translateText("高级终端设置"), "Advanced terminal settings");
  assert.equal(translateText("自定义字号"), "Custom font size");
  assert.equal(translateText("设置终端文字大小。"), "Set the terminal font size.");
  assert.equal(translateText("prod-db-01"), "prod-db-01");
  assert.equal(translateText("删除“prod-db-01”？"), "Delete “prod-db-01”?");
});

test("returns Chinese source text after switching back from English", () => {
  setLocale("en");
  assert.equal(translateText("未分组"), "Ungrouped");
  setLocale("zh-CN");
  assert.equal(translateText("未分组"), "未分组");
  assert.equal(getLocale(), "zh-CN");
});

test("localizes runtime confirmations and status text without translating server details", () => {
  setLocale("en");
  assert.equal(translateText("还有 3 个传输任务正在运行。断开连接会取消这些任务，是否继续？"), "3 transfers are still running. Disconnecting will cancel them. Continue?");
  assert.equal(translateText("将永久删除远程文件“配置.json”。"), "Permanently delete remote file “配置.json”.");
  assert.equal(translateText("连接失败。"), "Connection failed.");
  assert.equal(translateText("终端输出序号不连续，已停止接收。"), "Terminal output sequence is out of order; receiving has stopped.");
  assert.equal(translateText("终端数据长度校验失败，已停止接收。"), "Terminal data length check failed; receiving has stopped.");
  assert.equal(translateText("远程 Shell 已结束"), "Remote Shell ended");
  assert.equal(translateText("[无法读取终端状态：connection reset]"), "[Could not read terminal state: connection reset]");
  assert.equal(translateText("db-1 · 10.0.0.1:22\n凭据只会用于本次连接，不会写入日志。"), "db-1 · 10.0.0.1:22\nCredentials are used only for this connection and are not written to logs.");
  assert.equal(translateText("取消传输失败：transfer-1"), "Could not cancel transfer: transfer-1");
});

test("localizes jump-host and proxy settings and their validation messages", () => {
  setLocale("en");
  assert.equal(translateText("跳板机"), "Jump Host");
  assert.equal(translateText("代理"), "Proxy");
  assert.equal(translateText("无"), "None");
  assert.equal(translateText("跳板机格式无效，请使用 host 或 user@host。"), "Enter a jump host as host or user@host.");
  assert.equal(translateText("无法连接代理服务器，请检查代理地址、端口和网络。"), "Could not connect to the proxy. Check its address, port, and network.");
});

test("localizes separated appearance and language settings", () => {
  setLocale("en");
  assert.equal(translateText("选择 MauLink 的主题。默认跟随系统外观。"), "Choose the MauLink theme. Follows system appearance by default.");
  assert.equal(translateText("设置 MauLink 的界面语言偏好。"), "Choose the MauLink interface language.");
  assert.equal(translateText("简体中文"), "Simplified Chinese");
  assert.equal(translateText("English"), "English");
  setLocale("zh-CN");
});

test("localizes command palette actions and remote file limitations", () => {
  setLocale("en");
  assert.equal(translateText("输入命令或搜索…"), "Type a command or search…");
  assert.equal(translateText("打开文件与传输"), "Open Files and Transfers");
  assert.equal(translateText("清除已完成的传输"), "Clear Completed Transfers");
  assert.equal(translateText("远程文本预览和编辑暂不可用。"), "Remote text preview and editing are not available yet.");
  setLocale("zh-CN");
});
