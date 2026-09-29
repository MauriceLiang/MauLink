import test from "node:test";
import assert from "node:assert/strict";
import { getLocale, setLocale, translateText } from "../src/i18n.mjs";

test("translates interface labels while leaving user supplied values intact", () => {
  setLocale("en");
  assert.equal(translateText("添加服务器"), "Add Server");
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
