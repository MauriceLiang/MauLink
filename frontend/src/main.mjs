import { createTerminalOutputConsumer, encodeBytesBase64 } from "./terminal-codec.mjs";
import { joinRemotePath, parentRemotePath } from "./remote-path.mjs";
import { formatBytes, formatPercent, formatRate as formatMonitorRate, formatUptime, qualityLabel, sparklinePath } from "./monitor-view.mjs";
import { hydrateIcons, iconMarkup } from "./icons.mjs";
import { setLocale, translateText } from "./i18n.mjs";
import { filterPaletteCommands, movePaletteSelection } from "./command-palette.mjs";
import { copyTerminalSelection, isTerminalCopyOnSelectEnabled, setTerminalCopyOnSelectEnabled } from "./terminal-preferences.mjs";

const API_VERSION = 1;
const MAX_TERMINAL_INPUT_CHUNK = 64 * 1024;
const MAX_QUEUED_TERMINAL_INPUT = 256 * 1024;
const core = window.__TAURI__?.core;
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
let closeServerActionMenu = null;
let closeFileActionMenu = null;
const commandPaletteState = { commands: [], visible: [], activeIndex: -1, returnFocus: null, restoreFocus: true };
let terminalClipboardWarningShown = false;

const state = {
  profiles: [],
  groups: [],
  query: "",
  editingProfile: null,
  selectedKey: null,
  deleteTarget: null,
  connectionFailureProfile: null,
  restoreEditorAfterDeleteConfirmation: false,
  connections: new Map(),
  activeWorkspace: null,
  disconnecting: false,
  transferStarts: new Set(),
  activeChallenge: null,
  challengeId: null,
  pendingSecret: null,
  terminals: new Map(),
  terminalOrder: [],
  activeTerminalId: null,
  nextTerminalNumber: 1,
  workspaceMode: "terminal",
  files: {
    connectionId: null,
    path: "",
    entries: [],
    cursorId: null,
    selectedPath: null,
    loading: false,
    needsReload: true,
    sequence: 0,
    action: null,
    transfers: new Map(),
    transferOrder: [],
    dismissedTransferIds: new Set(),
  },
  monitor: {
    snapshot: null,
    histories: {},
    sequence: 0,
    pollingConnectionId: null,
    activityKey: null,
    lastHistoryAt: 0,
    error: null,
    refreshing: false,
  },
  currentPage: "servers",
  settings: {
    value: {
      theme: "system",
      language: "zh-CN",
      terminalFontFamily: "monospace",
      terminalFontSize: 14,
      terminalCursorStyle: "block",
      terminalScrollbackLines: 10_000,
      downloadDirectoryToken: null,
      confirmBeforeDisconnect: true,
    },
    revision: 0,
    updatedAtMs: null,
  },
  settingsSave: Promise.resolve(),
  appInfo: null,
  loading: false,
};

const knownErrors = {
  "errors.hostUnreachable": "无法访问这台服务器，请检查主机地址和网络。",
  "errors.connectionTimeout": "连接超时。请检查服务器是否在线以及 SSH 端口是否开放。",
  "errors.authenticationFailed": "认证失败，请检查用户名、密码或私钥口令。",
  "errors.hostKeyChanged": "服务器身份与已保存的指纹不一致。为保护连接，MauLink 已暂停连接。",
  "errors.credentialNotFound": "没有找到已保存的凭据，请在连接提示中输入。",
  "errors.serverInUse": "服务器正在连接中，暂时无法修改连接信息。",
  "errors.serverNameInvalid": "服务器名称不符合要求。",
  "errors.hostInvalid": "请输入有效的主机地址。",
  "errors.usernameInvalid": "请输入有效的用户名。",
  "errors.portInvalid": "端口必须在 1 到 65535 之间。",
  "errors.groupNotFound": "所选分组已不存在，请刷新后重试。",
  "errors.privateKeyUnreadable": "无法读取所选私钥，请检查文件权限或重新选择。",
  "errors.terminalConsumerStalled": "终端输出接收暂停，已关闭当前终端以保护连接。",
  "errors.pathNotFound": "远程路径不存在，目录内容可能已变化。",
  "errors.pathExists": "该远程路径已存在。",
  "errors.targetExists": "目标已存在，MauLink 不会覆盖它。",
  "errors.directoryNotEmpty": "目录不为空，请先处理其中的文件。",
  "errors.permissionDenied": "服务器拒绝了此文件操作，请检查远程权限。",
  "errors.sftpOperationFailed": "SFTP 操作失败，请刷新目录后重试。",
  "errors.sftpEntryTooLarge": "远程项目超出安全响应大小，无法显示。",
  "errors.unsupportedPathEncoding": "服务器返回了 MauLink 无法安全处理的文件名编码。",
  "errors.sftpTransferFileTypeUnsupported": "目前只支持传输常规文件。",
  "errors.sftpDeleteConfirmationRequired": "删除操作需要明确确认。",
  "errors.sftpEntryTypeChanged": "项目类型已变化，请刷新目录后再操作。",
  "errors.sftpDeleteTypeUnsupported": "不支持删除此类型的远程项目。",
  "errors.sftpRootOperationDenied": "不能对服务器根目录执行此操作。",
  "errors.sftpNameInvalid": "名称无效，请输入单个文件名。",
  "errors.sftpPathInvalid": "远程路径无效。",
  "errors.sftpCursorClosed": "目录列表已过期，请刷新后重试。",
  "errors.transferBusy": "当前连接或应用的传输额度已满，请等待现有任务结束。",
  "errors.transferOutcomeUnknown": "连接在发布文件时中断，结果尚不确定。请检查目标和临时文件后再重试。",
  "errors.publishUnsupported": "服务器不支持安全发布临时文件。临时文件已保留，未覆盖目标。",
  "errors.localDiskFull": "本机磁盘空间不足，下载未能完成。",
  "errors.localFileOperationFailed": "本机文件操作失败，请检查路径和磁盘状态。",
  "errors.localUploadFileInvalid": "请选择一个常规文件进行上传。",
  "errors.localUploadFileUnreadable": "无法读取所选上传文件。",
  "errors.localDownloadPathInvalid": "下载位置无效或所在目录不存在。",
  "errors.activeTransfersRequireConfirmation": "此连接仍有传输任务，请先取消任务并再次断开。",
  "errors.monitorTimeout": "服务器监控采集超时，请稍后重试。",
  "errors.monitorOutputTooLarge": "服务器监控输出超出安全限制。",
  "errors.monitorCollectionFailed": "无法读取服务器监控数据。",
  "errors.monitorHistoryRangeInvalid": "监控历史时间范围无效。",
  "errors.connectionNotReady": "SSH 连接尚未就绪。",
  "errors.proxyConnectionFailed": "无法连接代理服务器，请检查代理地址、端口和网络。",
  "errors.proxyHandshakeFailed": "代理拒绝或无法建立 SSH 隧道，请检查代理协议和目标访问权限。",
  "errors.proxyConfigurationInvalid": "代理配置不完整，请检查代理类型、主机和端口。",
  "errors.jumpHostInvalid": "跳板机格式无效，请使用 host 或 user@host。",
};

function request(payload = {}) {
  return { apiVersion: API_VERSION, requestId: crypto.randomUUID(), payload };
}

function invoke(command, payload = {}, extra = {}) {
  if (!core?.invoke) throw new Error("MauLink 本地服务不可用，请重新启动应用。");
  return core.invoke(command, { request: request(payload), ...extra });
}

function errorMessage(error) {
  if (typeof error === "string") return error;
  if (error?.messageKey && knownErrors[error.messageKey]) return translateText(knownErrors[error.messageKey]);
  if (error?.messageKey) return `${error.code ?? "操作失败"} · ${error.messageKey}`;
  if (error?.code) {
    const message = knownErrors[`errors.${error.code}`];
    return message ? translateText(message) : `${error.code}${error.stage ? ` · ${error.stage}` : ""}`;
  }
  if (error instanceof Error) return error.message;
  try { return JSON.stringify(error); } catch { return "发生未知错误。"; }
}

function toast(message, kind = "success", duration = 3600) {
  const region = $("#toast-region");
  const item = document.createElement("div");
  item.className = `toast${kind === "error" ? " is-error" : ""}`;
  const icon = document.createElement("span");
  icon.className = "toast-icon";
  icon.innerHTML = iconMarkup(kind === "error" ? "circle-alert" : "circle-check");
  const text = document.createElement("span");
  text.className = "toast-message";
  text.textContent = message;
  item.append(icon, text);
  region.append(item);
  window.setTimeout(() => item.remove(), duration);
}

function setBackendStatus(status, label) {
  const indicator = $("#backend-indicator");
  indicator.classList.toggle("is-ready", status === "ready");
  indicator.classList.toggle("is-error", status === "error");
  $("#backend-label").textContent = label;
}

function openDialog(dialog) {
  if (!dialog.open) dialog.showModal();
}

function closeDialog(dialog) {
  if (dialog.open) dialog.close();
}

function dismissServerActionMenu() {
  closeServerActionMenu?.();
  closeServerActionMenu = null;
}

function dismissFileActionMenu() {
  closeFileActionMenu?.();
  closeFileActionMenu = null;
}

function showServerActionMenu(profile, x, y, returnFocus) {
  dismissServerActionMenu();
  dismissFileActionMenu();
  const menu = document.createElement("div");
  menu.className = "server-context-menu";
  menu.setAttribute("role", "menu");
  menu.setAttribute("aria-label", translateText("服务器操作"));
  menu.style.left = `${Math.max(8, Math.min(x, window.innerWidth - 200))}px`;
  menu.style.top = `${Math.max(8, Math.min(y, window.innerHeight - 88))}px`;
  menu.innerHTML = `
    <button class="server-context-menu-item" type="button" role="menuitem" data-menu-action="edit">${iconMarkup("pencil")}<span>${translateText("编辑服务器")}</span></button>
    <button class="server-context-menu-item is-danger" type="button" role="menuitem" data-menu-action="delete">${iconMarkup("trash-2")}<span>${translateText("删除服务器")}</span></button>`;
  document.body.append(menu);

  const controller = new AbortController();
  const close = (restoreFocus = false) => {
    controller.abort();
    menu.remove();
    if (closeServerActionMenu === close) closeServerActionMenu = null;
    returnFocus.setAttribute("aria-expanded", "false");
    if (restoreFocus) returnFocus.focus();
  };
  closeServerActionMenu = close;
  document.addEventListener("pointerdown", (event) => {
    if (!menu.contains(event.target)) close();
  }, { signal: controller.signal });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close(true);
    }
  }, { signal: controller.signal });
  menu.addEventListener("click", (event) => {
    const action = event.target.closest("[data-menu-action]")?.dataset.menuAction;
    if (!action) return;
    close();
    if (action === "edit") openServerDialog(profile);
    else openDeleteDialog(profile);
  });
  menu.querySelector("button")?.focus();
}

function showFileActionMenu(entry, x, y, returnFocus) {
  dismissFileActionMenu();
  dismissServerActionMenu();
  const directory = entry.fileType === "directory";
  const menu = document.createElement("div");
  menu.className = "server-context-menu file-context-menu";
  menu.setAttribute("role", "menu");
  menu.setAttribute("aria-label", translateText("文件操作"));
  menu.style.left = `${Math.max(8, x)}px`;
  menu.style.top = `${Math.max(8, y)}px`;
  const actions = directory
    ? [
      { id: "open", label: "打开", icon: "folder" },
      { id: "copy", label: "复制路径", icon: "copy" },
      { id: "rename", label: "重命名", icon: "pencil" },
      { id: "delete", label: "删除", icon: "trash-2", danger: true },
    ]
    : [
      { id: "download", label: "下载", icon: "download", disabled: !["file", "symlink"].includes(entry.fileType) },
      { id: "view", label: "查看", icon: "eye", disabled: true, title: "远程文本预览和编辑暂不可用。" },
      { id: "edit", label: "编辑", icon: "pencil", disabled: true, title: "远程文本预览和编辑暂不可用。" },
      { id: "copy", label: "复制路径", icon: "copy" },
      { id: "rename", label: "重命名", icon: "pencil" },
      { id: "delete", label: "删除", icon: "trash-2", danger: true },
    ];
  for (const action of actions) {
    if (action.id === "delete") {
      const separator = document.createElement("div");
      separator.className = "server-context-menu-separator";
      separator.setAttribute("role", "separator");
      menu.append(separator);
    }
    const button = document.createElement("button");
    button.className = `server-context-menu-item${action.danger ? " is-danger" : ""}`;
    button.type = "button";
    button.setAttribute("role", "menuitem");
    button.dataset.fileMenuAction = action.id;
    button.disabled = Boolean(action.disabled || (directory && action.id === "open" && !state.activeWorkspace));
    if (action.title) button.title = translateText(action.title);
    const icon = document.createElement("span");
    icon.innerHTML = iconMarkup(action.icon);
    const label = document.createElement("span");
    label.textContent = translateText(action.label);
    button.append(icon.firstElementChild, label);
    menu.append(button);
  }
  document.body.append(menu);
  const menuBounds = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(8, Math.min(x, window.innerWidth - menuBounds.width - 8))}px`;
  menu.style.top = `${Math.max(8, Math.min(y, window.innerHeight - menuBounds.height - 8))}px`;

  const controller = new AbortController();
  const close = (restoreFocus = false) => {
    controller.abort();
    menu.remove();
    if (closeFileActionMenu === close) closeFileActionMenu = null;
    returnFocus.setAttribute("aria-expanded", "false");
    if (restoreFocus && returnFocus.isConnected) returnFocus.focus();
  };
  closeFileActionMenu = close;
  document.addEventListener("pointerdown", (event) => {
    if (!menu.contains(event.target)) close();
  }, { signal: controller.signal });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close(true);
    }
  }, { signal: controller.signal });
  menu.addEventListener("click", (event) => {
    const action = event.target.closest("[data-file-menu-action]")?.dataset.fileMenuAction;
    if (!action) return;
    close();
    if (action === "open" && directory) void openRemoteEntry(entry);
    else if (action === "download") void selectDownload();
    else if (action === "copy") void copySelectedPath();
    else if (action === "rename") openFileNameDialog("rename");
    else if (action === "delete") openFileDeleteDialog();
  });
  menu.querySelector("button:not(:disabled)")?.focus();
}

function buildPaletteCommands() {
  const navigation = translateText("导航");
  const actions = translateText("操作");
  const hasWorkspace = Boolean(state.activeWorkspace);
  const workspaceReady = state.activeWorkspace?.snapshot?.state === "ready" && !state.disconnecting;
  const hasCompletedTransfers = state.files.transferOrder.some((id) => state.files.transfers.get(id)?.state === "completed");
  const profiles = state.profiles.map((profile) => ({
    id: `server:${profile.id}`,
    group: translateText("服务器"),
    type: "server",
    profileId: profile.id,
    label: profile.name || profile.host,
    meta: `${profile.username}@${profile.host}`,
    icon: "server",
    keywords: [profile.name, profile.host, profile.username, "server", "connect"],
  }));
  return [
    ...profiles,
    { id: "workspace", group: navigation, type: "navigation", target: "workspace", label: translateText("打开工作区"), icon: "terminal", disabled: !hasWorkspace, keywords: ["workspace", "terminal"] },
    { id: "monitor", group: navigation, type: "navigation", target: "monitor", label: translateText("打开监控"), icon: "activity", disabled: !workspaceReady, keywords: ["monitor", "metrics"] },
    { id: "files", group: navigation, type: "navigation", target: "files", label: translateText("打开文件与传输"), icon: "folder", disabled: !workspaceReady, keywords: ["files", "transfers", "sftp"] },
    { id: "add-server", group: actions, type: "action", action: "add-server", label: translateText("添加新的服务器"), icon: "plus", keywords: ["add", "server"] },
    { id: "toggle-theme", group: actions, type: "action", action: "toggle-theme", label: translateText("切换浅色 / 深色主题"), icon: "moon", keywords: ["theme", "appearance", "dark", "light"] },
    { id: "toggle-language", group: actions, type: "action", action: "toggle-language", label: translateText("切换语言"), icon: "languages", keywords: ["language", "english", "chinese"] },
    { id: "clear-transfers", group: actions, type: "action", action: "clear-transfers", label: translateText("清除已完成的传输"), icon: "trash-2", disabled: !hasCompletedTransfers, keywords: ["clear", "completed", "transfers"] },
  ];
}

function syncPaletteSelection() {
  const input = $("#command-palette-input");
  const options = $$(".command-palette-option", $("#command-palette-results"));
  for (const option of options) {
    const selected = Number(option.dataset.paletteIndex) === commandPaletteState.activeIndex;
    option.classList.toggle("is-active", selected);
    option.setAttribute("aria-selected", String(selected));
  }
  const active = options.find((option) => Number(option.dataset.paletteIndex) === commandPaletteState.activeIndex);
  if (active) input.setAttribute("aria-activedescendant", active.id);
  else input.removeAttribute("aria-activedescendant");
}

function renderCommandPalette() {
  const input = $("#command-palette-input");
  const results = $("#command-palette-results");
  commandPaletteState.visible = filterPaletteCommands(commandPaletteState.commands, input.value);
  commandPaletteState.activeIndex = movePaletteSelection(commandPaletteState.visible, -1, 1);
  results.replaceChildren();
  if (!commandPaletteState.visible.length) {
    const empty = document.createElement("div");
    empty.className = "command-palette-empty";
    empty.textContent = translateText("没有匹配的命令");
    results.append(empty);
    syncPaletteSelection();
    return;
  }

  const groups = new Map();
  commandPaletteState.visible.forEach((command, index) => {
    if (!groups.has(command.group)) groups.set(command.group, []);
    groups.get(command.group).push({ command, index });
  });
  for (const [groupName, items] of groups) {
    const group = document.createElement("section");
    group.className = "command-palette-group";
    group.setAttribute("role", "group");
    group.setAttribute("aria-label", groupName);
    const heading = document.createElement("div");
    heading.className = "command-palette-group-label";
    heading.textContent = groupName;
    group.append(heading);
    for (const { command, index } of items) {
      const option = document.createElement("button");
      option.className = "command-palette-option";
      option.type = "button";
      option.setAttribute("role", "option");
      option.setAttribute("aria-selected", String(index === commandPaletteState.activeIndex));
      option.setAttribute("aria-disabled", String(Boolean(command.disabled)));
      option.id = `command-palette-option-${index}`;
      option.dataset.paletteIndex = String(index);
      option.disabled = Boolean(command.disabled);
      const icon = document.createElement("span");
      icon.innerHTML = iconMarkup(command.icon);
      const label = document.createElement("span");
      label.className = "command-palette-option-label";
      label.textContent = command.label;
      option.append(icon.firstElementChild, label);
      if (command.meta) {
        const meta = document.createElement("span");
        meta.className = "command-palette-option-meta";
        meta.textContent = command.meta;
        option.append(meta);
      }
      group.append(option);
    }
    results.append(group);
  }
  syncPaletteSelection();
}

function closeCommandPalette(restoreFocus = true) {
  const dialog = $("#command-palette");
  if (!dialog.open) return;
  commandPaletteState.restoreFocus = restoreFocus;
  dialog.close();
}

function openCommandPalette() {
  const dialog = $("#command-palette");
  if (dialog.open) return;
  dismissServerActionMenu();
  dismissFileActionMenu();
  if ($$("dialog[open]").length) return;
  if ($(".app-shell").classList.contains("is-focused")) setTerminalFocus(false);
  commandPaletteState.returnFocus = document.activeElement;
  commandPaletteState.restoreFocus = true;
  commandPaletteState.commands = buildPaletteCommands();
  $("#command-palette-input").value = "";
  $("#command-palette-input").setAttribute("aria-expanded", "true");
  renderCommandPalette();
  dialog.showModal();
  $("#command-palette-input").focus();
}

function activatePaletteCommand(command) {
  if (!command || command.disabled) return;
  closeCommandPalette(false);
  if (command.type === "server") {
    const profile = state.profiles.find((item) => item.id === command.profileId);
    if (profile) void startSavedConnection(profile, "workspace");
    return;
  }
  if (command.type === "navigation") {
    if (command.target === "workspace" && state.activeWorkspace) {
      showPage("workspace");
      setWorkspaceMode("terminal");
    } else if (command.target === "monitor" && state.activeWorkspace?.snapshot?.state === "ready") {
      showPage("workspace");
      setWorkspaceMode("monitor");
    } else if (command.target === "files" && state.activeWorkspace?.snapshot?.state === "ready") {
      showPage("workspace");
      setWorkspaceMode("files");
    }
    return;
  }
  if (command.action === "add-server") openServerDialog();
  else if (command.action === "toggle-theme") {
    const current = state.settings.value.theme;
    const effective = current === "system"
      ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
      : current;
    const next = effective === "dark" ? "light" : "dark";
    $(`[data-theme-choice="${next}"]`)?.click();
  } else if (command.action === "toggle-language") {
    const next = state.settings.value.language === "en" ? "zh-CN" : "en";
    $(`[data-language-choice="${next}"]`)?.click();
  } else if (command.action === "clear-transfers") clearCompletedTransfers();
}

function clearCompletedTransfers() {
  const completedIds = state.files.transferOrder.filter((id) => state.files.transfers.get(id)?.state === "completed");
  if (!completedIds.length) return;
  for (const id of completedIds) {
    state.files.dismissedTransferIds.add(id);
    state.files.transfers.delete(id);
  }
  state.files.transferOrder = state.files.transferOrder.filter((id) => !completedIds.includes(id));
  renderTransfers();
  toast("已清除已完成的传输。");
}

function showConnectionError(profile, error) {
  state.connectionFailureProfile = profile;
  setText($("#connection-error-server-name"), profile?.name || profile?.host || "SSH 服务器");
  setText($("#connection-error-address"), `${profile?.host ?? ""}:${profile?.port ?? 22}`);
  setText($("#connection-error-detail"), errorMessage(error ?? "SSH 连接失败。"));
  openDialog($("#connection-error-dialog"));
}

function setServerTestResult(profile, error = null) {
  const result = $("#test-result");
  const succeeded = error == null;
  result.hidden = false;
  result.dataset.state = succeeded ? "success" : "error";
  $("#test-result-icon").innerHTML = iconMarkup(succeeded ? "circle-check" : "circle-alert");
  setText($("#test-result-title"), succeeded ? "连接成功" : "连接失败");
  setText($("#test-result-detail"), succeeded
    ? profile?.authType === "privateKey" ? "SSH 密钥认证已完成。" : "密码认证已完成。"
    : errorMessage(error));
}

function groupLabel(groupId) {
  return state.groups.find((group) => group.id === groupId)?.name ?? "未分组";
}

function filteredProfiles() {
  const query = state.query.trim().toLocaleLowerCase();
  if (!query) return state.profiles;
  return state.profiles.filter((profile) =>
    [profile.name, profile.host, profile.username, groupLabel(profile.groupId)]
      .some((value) => value.toLocaleLowerCase().includes(query)));
}

function connectionFor(serverId) {
  return state.connections.get(serverId) ?? null;
}

function connectionCount() {
  return [...state.connections.values()].filter((entry) => entry.snapshot?.state === "ready").length;
}

function renderGroups() {
  const groupSelect = $("#profile-group");
  const selected = groupSelect.value;
  groupSelect.replaceChildren(new Option("未分组", ""));
  for (const group of state.groups) {
    const option = new Option(group.name, group.id);
    option.dataset.userContent = "true";
    groupSelect.add(option);
  }
  if (state.groups.some((group) => group.id === selected)) groupSelect.value = selected;
}

function setText(element, value) {
  element.textContent = value == null ? "" : String(value);
}

function updateTerminalFontSizeControls(value) {
  const terminalFontSize = Number(value);
  $("#setting-terminal-size").value = String(terminalFontSize);
  setText($("#setting-terminal-size-value"), `${terminalFontSize} px`);
  for (const choice of $$("[data-terminal-font-size]")) {
    choice.setAttribute("aria-pressed", String(Number(choice.dataset.terminalFontSize) === terminalFontSize));
  }
}

function renderSettings() {
  const settings = state.settings.value;
  document.documentElement.dataset.theme = settings.theme;
  setLocale(settings.language);
  $("#setting-confirm-disconnect").checked = settings.confirmBeforeDisconnect;
  $("#setting-terminal-font").value = settings.terminalFontFamily;
  updateTerminalFontSizeControls(settings.terminalFontSize);
  for (const choice of $$("#setting-terminal-cursor [data-terminal-cursor]")) {
    choice.setAttribute("aria-pressed", String(choice.dataset.terminalCursor === settings.terminalCursorStyle));
  }
  $("#setting-terminal-scrollback").value = String(settings.terminalScrollbackLines);
  $("#setting-terminal-copy-on-select").checked = isTerminalCopyOnSelectEnabled(localStorage);
  for (const choice of $$("[data-theme-choice]")) {
    choice.setAttribute("aria-pressed", String(choice.dataset.themeChoice === settings.theme));
  }
  for (const choice of $$("[data-language-choice]")) {
    choice.setAttribute("aria-pressed", String(choice.dataset.languageChoice === settings.language));
  }
}

function applyTerminalSettings(settings) {
  for (const terminal of state.terminals.values()) {
    terminal.instance.options.fontFamily = settings.terminalFontFamily;
    terminal.instance.options.fontSize = settings.terminalFontSize;
    terminal.instance.options.cursorStyle = settings.terminalCursorStyle;
    terminal.instance.options.scrollback = settings.terminalScrollbackLines;
    if (state.currentPage === "workspace" && state.workspaceMode === "terminal" && terminal.terminalId === state.activeTerminalId) {
      fitTerminal(terminal);
    }
  }
}

function saveSettings(patch) {
  const operation = state.settingsSave.then(async () => {
    const value = { ...state.settings.value, ...patch };
    setText($("#settings-save-state"), "正在保存…");
    try {
      state.settings = await invoke("settings_update", {
        expectedRevision: state.settings.revision,
        value,
      });
      applyTerminalSettings(state.settings.value);
      setText($("#settings-save-state"), "已保存");
    } catch (error) {
      renderSettings();
      setText($("#settings-save-state"), "保存失败");
      toast(`设置未能保存：${errorMessage(error)}`, "error", 7000);
    }
  });
  state.settingsSave = operation.catch(() => {});
  return operation;
}

function renderStats() {
  setText($("#stat-total"), state.profiles.length);
  setText($("#stat-connected"), connectionCount());
  setText($("#stat-groups"), state.groups.length);
}

function renderConnectionStatus(profile) {
  const entry = connectionFor(profile.id);
  const homeCard = $(`.home-server-card[data-server-id="${CSS.escape(profile.id)}"]`);
  if (homeCard) {
    const state = entry?.snapshot?.state;
    const ready = state === "ready";
    const pending = Boolean(entry && !["ready", "failed", "closed", "cancelled"].includes(state));
    const status = $(".home-server-status-dot", homeCard);
    status.classList.toggle("is-ready", ready);
    status.classList.toggle("is-pending", pending);
    status.setAttribute("aria-label", ready ? "已连接" : pending ? connectionStateLabel(state) : "未连接");
    const button = $(".home-server-open", homeCard);
    button.textContent = ready ? "打开工作区" : pending ? "连接中…" : "打开";
    button.disabled = pending;
  }
  const row = $(`.server-row[data-server-id="${CSS.escape(profile.id)}"]`);
  if (!row) return;
  const stateValue = entry?.snapshot?.state;
  const pending = Boolean(entry && !["ready", "failed", "closed", "cancelled"].includes(stateValue));
  row.classList.toggle("is-selected", state.activeWorkspace?.profile?.id === profile.id || pending);
  const status = $(".server-connection", row);
  const button = $("[data-action='open-server']", row);
  status.classList.toggle("is-ready", stateValue === "ready");
  status.classList.toggle("is-pending", pending);
  const statusLabel = stateValue === "ready"
    ? "已连接"
    : pending
      ? connectionStateLabel(stateValue)
      : "未连接";
  const statusText = $(".server-status-label", status);
  status.setAttribute("aria-label", statusLabel);
  if (statusText) statusText.textContent = statusLabel;
  button.setAttribute("aria-label", `${statusLabel === "未连接" ? "打开" : statusLabel} ${profile.name || profile.host}`);
  button.disabled = pending;
}

function connectionStateLabel(value) {
  const labels = {
    created: "准备连接", resolving: "解析主机", connecting: "正在连接", verifyingHostKey: "验证服务器",
    awaitingHostTrust: "等待身份确认", awaitingCredentials: "等待凭据", authenticating: "身份验证中", disconnecting: "正在断开",
  };
  return labels[value] ?? "连接中";
}

function makeEmptyState(title, description, actionLabel, action) {
  const box = document.createElement("div");
  box.className = "empty-state";
  const symbol = document.createElement("div");
  symbol.className = "empty-symbol";
  symbol.innerHTML = iconMarkup("server");
  const heading = document.createElement("h3");
  heading.textContent = title;
  const text = document.createElement("p");
  text.textContent = description;
  box.append(symbol, heading, text);
  if (actionLabel) {
    const button = document.createElement("button");
    button.className = "button button-secondary";
    button.type = "button";
    button.textContent = actionLabel;
    button.addEventListener("click", action);
    box.append(button);
  }
  return box;
}

function createServerRow(profile) {
  const row = document.createElement("article");
  row.className = "server-row";
  row.dataset.serverId = profile.id;
  row.innerHTML = `
    <button class="server-main" type="button" data-action="open-server">
      <span class="server-connection" role="img" aria-label="未连接"><i></i><span class="sr-only server-status-label">未连接</span></span>
      <span class="server-details"><span class="server-name"></span><span class="server-address"></span></span>
    </button>
    <button class="server-menu-trigger" type="button" aria-label="${translateText("更多操作")}" aria-haspopup="menu" aria-expanded="false" title="${translateText("更多操作")}">${iconMarkup("ellipsis")}</button>`;
  hydrateIcons(row);
  for (const selector of [".server-name", ".server-address"]) $(selector, row).dataset.userContent = "true";
  setText($(".server-name", row), profile.name || profile.host);
  setText($(".server-address", row), `${profile.username}@${profile.host}`);

  $("[data-action='open-server']", row).addEventListener("click", () => void startSavedConnection(profile, "workspace"));
  const menuTrigger = $(".server-menu-trigger", row);
  menuTrigger.addEventListener("click", (event) => {
    event.stopPropagation();
    const bounds = menuTrigger.getBoundingClientRect();
    menuTrigger.setAttribute("aria-expanded", "true");
    showServerActionMenu(profile, bounds.right - 192, bounds.bottom + 4, menuTrigger);
  });
  row.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    menuTrigger.setAttribute("aria-expanded", "true");
    showServerActionMenu(profile, event.clientX, event.clientY, menuTrigger);
  });
  return row;
}

function createServerGroup(groupId, name, profiles) {
  const section = document.createElement("section");
  section.className = "sidebar-group";
  const heading = document.createElement("div");
  heading.className = "sidebar-group-heading";
  const label = document.createElement("span");
  label.className = "sidebar-group-name";
  label.textContent = name;
  if (groupId) label.dataset.userContent = "true";
  const count = document.createElement("span");
  count.className = "sidebar-group-count";
  count.textContent = String(profiles.length);
  const addServer = document.createElement("button");
  addServer.className = "icon-button sidebar-group-add";
  addServer.type = "button";
  const addLabel = `${translateText("添加服务器")} · ${name}`;
  addServer.setAttribute("aria-label", addLabel);
  addServer.title = addLabel;
  addServer.innerHTML = iconMarkup("plus");
  addServer.addEventListener("click", () => openServerDialog(null, groupId));
  heading.append(label, count, addServer);

  const servers = document.createElement("div");
  servers.className = "sidebar-group-servers";
  for (const profile of profiles) servers.append(createServerRow(profile));
  section.append(heading, servers);
  return section;
}

function createHomeServerCard(profile) {
  const card = document.createElement("article");
  card.className = "home-server-card";
  card.dataset.serverId = profile.id;

  const status = document.createElement("span");
  status.className = "home-server-status-dot";
  status.setAttribute("role", "img");
  status.setAttribute("aria-label", "未连接");

  const details = document.createElement("div");
  details.className = "home-server-details";
  const name = document.createElement("div");
  name.className = "home-server-name";
  name.dataset.userContent = "true";
  name.textContent = profile.name || profile.host;
  const address = document.createElement("div");
  address.className = "home-server-address";
  address.dataset.userContent = "true";
  address.textContent = `${profile.username}@${profile.host}`;
  details.append(name, address);

  const button = document.createElement("button");
  button.className = "button button-secondary home-server-open";
  button.type = "button";
  button.textContent = "打开";
  button.addEventListener("click", () => void startSavedConnection(profile, "workspace"));

  card.append(status, details, button);
  return card;
}

function renderHomeServers() {
  const list = $("#server-home-list");
  list.replaceChildren(...state.profiles.map(createHomeServerCard));
  for (const profile of state.profiles) renderConnectionStatus(profile);
}

function renderServers() {
  const list = $("#server-list");
  list.replaceChildren();
  const items = filteredProfiles();
  $("#server-search").value = state.query;
  $("#global-search").value = state.query;
  if (items.length) {
    const knownGroupIds = new Set(state.groups.map((group) => group.id));
    for (const group of state.groups) {
      const profiles = items.filter((profile) => profile.groupId === group.id);
      if (profiles.length) list.append(createServerGroup(group.id, group.name, profiles));
    }
    const ungrouped = items.filter((profile) => !profile.groupId || !knownGroupIds.has(profile.groupId));
    if (ungrouped.length) list.append(createServerGroup("", "未分组", ungrouped));
    for (const profile of items) renderConnectionStatus(profile);
    return;
  }
  if (state.profiles.length === 0) {
    list.append(makeEmptyState("还没有服务器", "添加一台服务器，保存安全凭据后即可进入远程工作区。", "添加服务器", () => openServerDialog()));
  } else {
    list.append(makeEmptyState("没有找到匹配项", "换一个名称、主机地址或用户名试试。", null, null));
  }
}

function renderAll() {
  const hasProfiles = state.profiles.length > 0;
  document.body.classList.toggle("has-profiles", hasProfiles);
  $("#welcome-card").hidden = hasProfiles;
  $("#server-home-view").hidden = !hasProfiles;
  setText($("#welcome-title"), hasProfiles ? "选择一台服务器" : "还没有服务器");
  setText($("#welcome-description"), hasProfiles
    ? "从左侧选择服务器，即可进入远程工作区。"
    : "连接你的第一台服务器。\n在一个工作区中使用终端、管理文件并查看服务器状态。");
  $("#welcome-add-server").hidden = hasProfiles;
  renderHomeServers();
  renderGroups();
  renderStats();
  renderServers();
}

async function loadData() {
  state.loading = true;
  try {
    const groups = await invoke("group_list", {});
    state.groups = groups;
    const profiles = [];
    let cursor = null;
    do {
      const page = await invoke("server_list", { query: null, groupId: null, limit: 200, cursor });
      profiles.push(...page.items);
      cursor = page.nextCursor;
    } while (cursor);
    state.profiles = profiles;
    renderAll();
    setBackendStatus("ready", "本地数据已安全加载");
  } catch (error) {
    setBackendStatus("error", "本地服务暂不可用");
    toast(errorMessage(error), "error", 7000);
    throw error;
  } finally {
    state.loading = false;
  }
}

function openServerDialog(profile = null, groupId = "") {
  state.editingProfile = profile;
  state.selectedKey = null;
  const form = $("#server-form");
  form.reset();
  setText($("#server-dialog-title"), profile ? "编辑服务器" : "添加服务器");
  setText($("#server-dialog-description"), profile ? "更新 SSH 服务器连接信息" : "连接到一台新的 SSH 服务器");
  $("#server-cancel-button").hidden = false;
  $("#save-server").hidden = !profile;
  $("#save-server").textContent = "保存更改";
  $("#test-connection").hidden = Boolean(profile);
  $("#save-and-connect").hidden = Boolean(profile);
  $("#save-and-connect").textContent = profile ? "保存并连接" : "连接";
  $("#profile-delete-row").hidden = !profile;
  $("#test-result").hidden = true;
  $("#test-result").removeAttribute("data-state");
  $("#profile-name").value = profile?.name ?? "";
  $("#profile-host").value = profile?.host ?? "";
  $("#profile-port").value = String(profile?.port ?? 22);
  $("#profile-user").value = profile?.username ?? "";
  $("#profile-auth").value = profile?.authType ?? "password";
  $("#profile-secret").value = "";
  $("#profile-secret").type = "password";
  $("#toggle-secret-visibility").setAttribute("aria-label", "显示密码");
  $("#toggle-secret-visibility").setAttribute("aria-pressed", "false");
  $("#toggle-secret-visibility").innerHTML = iconMarkup("eye");
  $("#profile-group").value = profile?.groupId ?? groupId;
  $("#keepalive").value = String(profile?.keepaliveIntervalSeconds ?? 30);
  $("#jump-host").value = profile?.jumpHost ?? "";
  $("#jump-port").value = String(profile?.jumpPort ?? 22);
  $("#proxy-type").value = profile?.proxyType ?? "";
  $("#proxy-host").value = profile?.proxyHost ?? "";
  $("#proxy-port").value = profile?.proxyPort ? String(profile.proxyPort) : "";
  $("#remember-credential").checked = true;
  $("#clear-credential").checked = false;
  $("#remember-row").hidden = false;
  $("#clear-credential-row").hidden = !profile;
  $("#profile-secret").placeholder = profile?.hasSavedCredential ? "留空以保留当前凭据" : "输入 SSH 密码";
  $("#selected-key").textContent = profile?.authType === "privateKey" && profile.hasPrivateKey ? "当前私钥将在未重新选择时保留" : "尚未选择文件";
  $("#clear-key").hidden = true;
  $(".advanced-fields").open = false;
  hideFormError();
  updateProxyFields();
  updateAuthFields();
  openDialog($("#server-dialog"));
  window.setTimeout(() => $("#profile-host").focus(), 30);
}

function updateAuthFields() {
  const privateKey = $("#profile-auth").value === "privateKey";
  for (const choice of $$('[data-auth-type-choice]')) {
    choice.setAttribute("aria-pressed", String(choice.dataset.authTypeChoice === $("#profile-auth").value));
  }
  $("#key-picker").hidden = !privateKey;
  $("#secret-field").hidden = false;
  setText($("#secret-label"), privateKey ? "私钥口令（可选）" : "密码");
  $("#profile-secret").placeholder = privateKey ? "未加密私钥可留空" : "输入 SSH 密码";
  $("#remember-row").hidden = false;
  if (!privateKey) {
    state.selectedKey = null;
    $("#selected-key").textContent = "尚未选择文件";
    $("#clear-key").hidden = true;
  }
}

function updateProxyFields() {
  const enabled = Boolean($("#proxy-type").value);
  $("#proxy-fields").hidden = !enabled;
  $("#proxy-host").required = enabled;
  $("#proxy-port").required = enabled;
}

function hideFormError() {
  $("#server-form-error").hidden = true;
  $("#server-form-error").textContent = "";
}

function showFormError(message) {
  const error = $("#server-form-error");
  error.textContent = message;
  error.hidden = false;
}

function readProfileDraft() {
  const host = $("#profile-host").value.trim();
  const username = $("#profile-user").value.trim();
  const port = Number($("#profile-port").value);
  const keepalive = Number($("#keepalive").value);
  const jumpHost = $("#jump-host").value.trim();
  const jumpPortValue = $("#jump-port").value.trim();
  const jumpPort = jumpPortValue ? Number(jumpPortValue) : 22;
  const proxyType = $("#proxy-type").value || null;
  const proxyHost = $("#proxy-host").value.trim();
  const proxyPort = Number($("#proxy-port").value);
  const authType = $("#profile-auth").value;
  if (!host) throw new Error("请输入主机地址。");
  if (!username) throw new Error("请输入 SSH 用户名。");
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("端口必须是 1 到 65535 之间的整数。");
  if (!Number.isInteger(keepalive) || keepalive < 5 || keepalive > 300) throw new Error(translateText("保活间隔需在 5 到 300 秒之间。"));
  if (jumpHost && (!Number.isInteger(jumpPort) || jumpPort < 1 || jumpPort > 65535)) throw new Error(translateText("跳板机端口必须在 1 到 65535 之间。"));
  if (proxyType && (!proxyHost || !Number.isInteger(proxyPort) || proxyPort < 1 || proxyPort > 65535)) throw new Error(translateText("请输入有效的代理主机和端口。"));
  if (authType === "privateKey" && !state.selectedKey && !state.editingProfile?.hasPrivateKey) throw new Error("请选择 SSH 私钥文件。");
  return {
    name: $("#profile-name").value.trim() || null,
    host,
    port,
    username,
    authType,
    privateKeyToken: authType === "privateKey" ? state.selectedKey?.token ?? null : null,
    groupId: $("#profile-group").value || null,
    connectTimeoutMs: state.editingProfile?.connectTimeoutMs ?? 15_000,
    keepaliveIntervalSeconds: keepalive,
    jumpHost: jumpHost || null,
    jumpPort,
    proxyType,
    proxyHost: proxyType ? proxyHost : null,
    proxyPort: proxyType ? proxyPort : null,
  };
}

function connectionIdentityChanged(profile, draft) {
  return profile.host !== draft.host
    || profile.port !== draft.port
    || profile.username !== draft.username
    || profile.authType !== draft.authType
    || profile.connectTimeoutMs !== draft.connectTimeoutMs
    || profile.keepaliveIntervalSeconds !== draft.keepaliveIntervalSeconds
    || profile.jumpHost !== draft.jumpHost
    || profile.jumpPort !== draft.jumpPort
    || profile.proxyType !== draft.proxyType
    || profile.proxyHost !== draft.proxyHost
    || profile.proxyPort !== draft.proxyPort
    || Boolean(draft.privateKeyToken);
}

function credentialUpdate(profile, secret) {
  const remember = $("#remember-credential").checked;
  const clear = $("#clear-credential").checked;
  if (clear) return { mode: "clear" };
  if (secret && remember) return { mode: "replace", secret };
  if (secret && !remember && profile?.hasSavedCredential) {
    throw new Error("已有凭据仍保存在系统安全存储中。若只在本次使用新凭据，请先选择“移除已保存的凭据”。");
  }
  if (profile && connectionIdentityChanged(profile, readProfileDraft())) {
    throw new Error("连接身份已变化。请提供新的凭据并选择安全保存，或明确移除旧凭据后继续。");
  }
  if (profile) return { mode: "keep" };
  return { mode: "clear" };
}

async function saveProfile(connectAfterSave = false) {
  hideFormError();
  let draft;
  let result;
  let useSecretForThisConnection = false;
  const profile = state.editingProfile;
  const secret = $("#profile-secret").value;
  try {
    draft = readProfileDraft();
    const credential = credentialUpdate(profile, secret);
    useSecretForThisConnection = Boolean(secret) && (!$("#remember-credential").checked || credential.mode === "clear");
    if (profile) {
      result = await invoke("server_update", {
        serverId: profile.id,
        expectedRevision: profile.revision,
        profile: draft,
        credential,
      });
    } else {
      result = await invoke("server_create", { profile: draft, credential });
    }
  } catch (error) {
    showFormError(errorMessage(error));
    return;
  }

  const pendingSecret = connectAfterSave && useSecretForThisConnection ? secret : null;
  closeDialog($("#server-dialog"));
  try {
    await loadData();
  } catch {
    // The mutation has already succeeded; the next explicit refresh can recover the list.
  }
  const savedProfile = result.server;
  toast(profile ? "服务器配置已更新。" : "服务器已添加。");
  if (connectAfterSave) void startSavedConnection(savedProfile, "workspace", pendingSecret);
}

async function selectPrivateKey() {
  try {
    const selected = await invoke("local_file_select", { purpose: "privateKey" });
    if (!selected) return;
    state.selectedKey = selected;
    $("#selected-key").textContent = `${selected.displayName} · 仅本次有效`;
    $("#clear-key").hidden = false;
  } catch (error) {
    showFormError(errorMessage(error));
  }
}

function sameConnectionSettings(profile, draft) {
  return profile.host === draft.host
    && profile.port === draft.port
    && profile.username === draft.username
    && profile.authType === draft.authType
    && profile.connectTimeoutMs === draft.connectTimeoutMs
    && profile.keepaliveIntervalSeconds === draft.keepaliveIntervalSeconds
    && profile.jumpHost === draft.jumpHost
    && profile.jumpPort === draft.jumpPort
    && profile.proxyType === draft.proxyType
    && profile.proxyHost === draft.proxyHost
    && profile.proxyPort === draft.proxyPort
    && !state.selectedKey;
}

async function testProfileConnection() {
  hideFormError();
  $("#test-result").hidden = true;
  let draft;
  try {
    draft = readProfileDraft();
    if (state.editingProfile && !$("#profile-secret").value && sameConnectionSettings(state.editingProfile, draft)) {
      await startSavedConnection(state.editingProfile, "test", null, true);
      return;
    }
    if (state.editingProfile?.authType === "privateKey" && draft.authType === "privateKey" && !state.selectedKey) {
      throw new Error("测试已修改的私钥连接信息时，请重新选择私钥文件。");
    }
    await startDraftTest(draft, $("#profile-secret").value || null);
  } catch (error) {
    showFormError(errorMessage(error));
  }
}

function activeConnectionExists(serverId) {
  const entry = connectionFor(serverId);
  return entry && !["failed", "closed", "cancelled"].includes(entry.snapshot?.state);
}

async function startSavedConnection(profile, mode, pendingSecret = null, showTestResult = false) {
  if (activeChallengeExists()) {
    toast("请先完成当前的安全确认，再开始另一个连接。", "error");
    return;
  }
  if (mode === "workspace" && activeConnectionExists(profile.id)) {
    const entry = connectionFor(profile.id);
    if (entry.snapshot?.state === "ready") enterWorkspace(entry);
    else toast("这台服务器正在连接。", "error");
    return;
  }
  if (mode === "test" && activeConnectionExists(profile.id)) {
    toast("请先断开当前会话，再测试连接。", "error");
    return;
  }
  try {
    const snapshot = await invoke("connection_start", {
      source: { kind: "saved", serverId: profile.id, expectedRevision: profile.revision },
      mode,
    });
    const entry = {
      profile,
      serverId: profile.id,
      connectionId: snapshot.connectionId,
      snapshot,
      mode,
      pendingSecret,
      showTestResult,
      workspaceOpened: false,
      rememberAfterReady: null,
      watching: true,
    };
    if (mode === "workspace") state.connections.set(profile.id, entry);
    renderConnectionStatus(profile);
    renderStats();
    void watchConnection(entry);
  } catch (error) {
    if (mode === "workspace") showConnectionError(profile, error);
    else if (showTestResult) setServerTestResult(profile, error);
    else toast(errorMessage(error), "error", 7000);
  }
}

async function startDraftTest(profile, secret) {
  if (activeChallengeExists()) {
    toast("请先完成当前的安全确认，再开始另一个连接。", "error");
    return;
  }
  try {
    const snapshot = await invoke("connection_start", {
      source: { kind: "draft", profile, credential: secret },
      mode: "test",
    });
    const entry = {
      profile: {
        ...profile,
        id: state.editingProfile?.id ?? null,
        revision: state.editingProfile?.revision ?? null,
        hasSavedCredential: state.editingProfile?.hasSavedCredential ?? false,
      },
      serverId: state.editingProfile?.id ?? null,
      connectionId: snapshot.connectionId,
      snapshot,
      mode: "test",
      pendingSecret: null,
      showTestResult: true,
      workspaceOpened: false,
      rememberAfterReady: null,
      watching: true,
    };
    void watchConnection(entry);
  } catch (error) {
    setServerTestResult(profile, error);
  }
}

function activeChallengeExists() {
  return state.activeChallenge != null;
}

async function watchConnection(entry) {
  while (entry.watching) {
    try {
      const snapshot = await invoke("connection_get", { connectionId: entry.connectionId });
      entry.snapshot = snapshot;
      if (entry.serverId && entry.mode === "workspace") renderConnectionStatus(entry.profile);
      renderStats();
      if (snapshot.state === "awaitingHostTrust" && snapshot.hostKeyChallenge) {
        presentHostKeyChallenge(entry, snapshot.hostKeyChallenge);
      } else if (snapshot.state === "awaitingCredentials" && snapshot.authenticationChallenge) {
        if (entry.pendingSecret) {
          const secret = entry.pendingSecret;
          entry.pendingSecret = null;
          await submitCredential(entry, snapshot.authenticationChallenge, secret, false);
        } else {
          presentAuthenticationChallenge(entry, snapshot.authenticationChallenge);
        }
      } else if (snapshot.state === "ready") {
        if (entry.mode === "workspace" && !entry.workspaceOpened) {
          entry.workspaceOpened = true;
          if (entry.rememberAfterReady) void persistChallengeCredential(entry);
          enterWorkspace(entry);
        }
      } else if (["closed", "failed", "cancelled"].includes(snapshot.state)) {
        entry.watching = false;
        finishConnection(entry);
        return;
      }
      await delay(snapshot.state === "ready" ? 1400 : 300);
    } catch (error) {
      entry.watching = false;
      if (entry.mode === "workspace" && entry.serverId) state.connections.delete(entry.serverId);
      renderAll();
      toast(errorMessage(error), "error", 7000);
      return;
    }
  }
}

function finishConnection(entry) {
  if (entry.mode === "test") {
    const succeeded = entry.snapshot.state === "closed" && !entry.snapshot.error;
    if (entry.showTestResult) {
      if (succeeded) setServerTestResult(entry.profile);
      else setServerTestResult(entry.profile, entry.snapshot.error ?? `连接${entry.snapshot.state === "cancelled" ? "已取消" : "失败"}。`);
    } else if (succeeded) toast("连接测试成功，SSH 身份验证已完成。", "success");
    else toast(errorMessage(entry.snapshot.error ?? `连接${entry.snapshot.state === "cancelled" ? "已取消" : "失败"}。`), "error", 7000);
    if (state.activeChallenge?.entry === entry) {
      state.activeChallenge = null;
      closeDialog($("#challenge-dialog"));
    }
    return;
  }
  if (entry.serverId && state.connections.get(entry.serverId) === entry) state.connections.delete(entry.serverId);
  if (state.activeWorkspace === entry) {
    state.activeWorkspace = null;
    closeAllTerminals();
    showPage("servers");
  }
  renderAll();
  renderStats();
  if (entry.snapshot.state === "failed") showConnectionError(entry.profile, entry.snapshot.error);
  else if (entry.snapshot.state === "closed") toast("SSH 连接已断开。", "success");
}

async function cancelConnection(entry) {
  try {
    await invoke("connection_cancel", { connectionId: entry.connectionId });
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

function addFingerprintLine(container, label, value) {
  const line = document.createElement("div");
  line.className = "fingerprint-line";
  const caption = document.createElement("span");
  caption.textContent = label;
  const fingerprint = document.createElement("strong");
  fingerprint.textContent = value || "—";
  line.append(caption, fingerprint);
  container.append(line);
}

function presentHostKeyChallenge(entry, challenge) {
  if (state.activeChallenge?.id === challenge.challengeId) return;
  if (state.activeChallenge && state.activeChallenge.entry !== entry) return;
  state.activeChallenge = { entry, type: "hostKey", id: challenge.challengeId, challenge };
  state.challengeId = challenge.challengeId;
  const changed = Boolean(challenge.previousFingerprintSha256);
  $("#challenge-symbol").textContent = changed ? "!" : "◇";
  $("#challenge-symbol").classList.toggle("is-danger", changed);
  setText($("#challenge-kicker"), changed ? "HOST IDENTITY CHANGED" : "FIRST CONNECTION");
  setText($("#challenge-title"), changed ? "服务器身份发生变化" : "确认服务器身份");
  setText($("#challenge-intro"), changed
    ? `已保存的服务器指纹与当前收到的指纹不一致。请先核实服务器变更；在确认前，MauLink 会保持连接暂停。\n${challenge.host}:${challenge.port}`
    : `这是 MauLink 第一次连接这台服务器。核对指纹并选择是否信任。\n${challenge.host}:${challenge.port}`);
  const fingerprints = $("#fingerprint-block");
  fingerprints.replaceChildren();
  if (changed) addFingerprintLine(fingerprints, "此前保存", challenge.previousFingerprintSha256);
  addFingerprintLine(fingerprints, "当前指纹", `${challenge.algorithm} · ${challenge.fingerprintSha256}`);
  fingerprints.hidden = false;
  const warning = $("#challenge-warning");
  warning.hidden = !changed;
  warning.textContent = "指纹变化可能来自服务器重装，也可能表示连接目标已被替换。除非你已通过独立渠道核实，不要更新信任记录。";
  $("#challenge-auth-form").hidden = true;
  $("#auth-actions").hidden = true;
  $("#host-key-actions").hidden = false;
  $("#trust-host-once").hidden = changed;
  $("#trust-host-save").hidden = changed;
  $("#host-key-advanced").hidden = !changed;
  $("#host-key-advanced").open = false;
  openDialog($("#challenge-dialog"));
}

function presentAuthenticationChallenge(entry, challenge) {
  if (state.activeChallenge?.id === challenge.challengeId) return;
  if (state.activeChallenge && state.activeChallenge.entry !== entry) return;
  state.activeChallenge = { entry, type: "auth", id: challenge.challengeId, challenge };
  state.challengeId = challenge.challengeId;
  setText($("#challenge-symbol"), "⌑");
  $("#challenge-symbol").classList.remove("is-danger");
  setText($("#challenge-kicker"), "SSH AUTHENTICATION");
  setText($("#challenge-title"), challenge.credentialKind === "passphrase" ? "输入私钥口令" : "输入 SSH 密码");
  setText($("#challenge-intro"), `${entry.profile?.username ?? "当前用户"} · ${entry.profile?.host ?? "SSH 服务器"}\n凭据只会用于本次连接，不会写入日志。`);
  $("#fingerprint-block").hidden = true;
  $("#challenge-warning").hidden = true;
  $("#host-key-actions").hidden = true;
  $("#host-key-advanced").hidden = true;
  $("#challenge-auth-form").hidden = false;
  $("#challenge-secret").value = "";
  $("#challenge-secret").autocomplete = challenge.credentialKind === "password" ? "current-password" : "off";
  setText($("#challenge-secret-label"), challenge.credentialKind === "passphrase" ? "私钥口令" : "密码");
  $("#challenge-remember-row").hidden = !entry.profile || entry.mode === "test";
  $("#challenge-remember").checked = false;
  $("#challenge-error").hidden = true;
  $("#auth-actions").hidden = false;
  openDialog($("#challenge-dialog"));
  window.setTimeout(() => $("#challenge-secret").focus(), 20);
}

async function respondHostKey(decision) {
  const active = state.activeChallenge;
  if (!active || active.type !== "hostKey") return;
  try {
    await invoke("host_key_respond", { connectionId: active.entry.connectionId, challengeId: active.id, decision });
    state.activeChallenge = null;
    closeDialog($("#challenge-dialog"));
  } catch (error) {
    toast(errorMessage(error), "error", 6000);
  }
}

async function submitCredential(entry, challenge, secret, remember) {
  await invoke("auth_respond", { connectionId: entry.connectionId, challengeId: challenge.challengeId, secret });
  if (remember && entry.profile && entry.mode === "workspace") entry.rememberAfterReady = secret;
  if (state.activeChallenge?.entry === entry) {
    state.activeChallenge = null;
    closeDialog($("#challenge-dialog"));
  }
}

async function submitAuthentication() {
  const active = state.activeChallenge;
  if (!active || active.type !== "auth") return;
  const secret = $("#challenge-secret").value;
  if (!secret) {
    $("#challenge-error").textContent = "请输入凭据后继续。";
    $("#challenge-error").hidden = false;
    return;
  }
  const remember = !$("#challenge-remember-row").hidden && $("#challenge-remember").checked;
  try {
    await submitCredential(active.entry, active.challenge, secret, remember);
    $("#challenge-secret").value = "";
  } catch (error) {
    $("#challenge-error").textContent = errorMessage(error);
    $("#challenge-error").hidden = false;
  }
}

async function persistChallengeCredential(entry) {
  const secret = entry.rememberAfterReady;
  entry.rememberAfterReady = null;
  if (!secret || !entry.profile) return;
  try {
    const result = await invoke("server_update", {
      serverId: entry.profile.id,
      expectedRevision: entry.profile.revision,
      profile: {
        name: entry.profile.name,
        host: entry.profile.host,
        port: entry.profile.port,
        username: entry.profile.username,
        authType: entry.profile.authType,
        privateKeyToken: null,
        groupId: entry.profile.groupId,
        connectTimeoutMs: entry.profile.connectTimeoutMs,
        keepaliveIntervalSeconds: entry.profile.keepaliveIntervalSeconds,
      },
      credential: { mode: "replace", secret },
    });
    entry.profile = result.server;
    await loadData();
  } catch (error) {
    toast(`连接成功，但凭据未能保存：${errorMessage(error)}`, "error", 7000);
  }
}

function showPage(page) {
  if (page !== "workspace" && $(".app-shell").classList.contains("is-focused")) setTerminalFocus(false);
  $(".app-shell").classList.toggle("is-settings", page === "settings");
  state.currentPage = page;
  if (page !== "workspace" && (state.files.cursorId || state.files.loading)) {
    state.files.sequence += 1;
    state.files.needsReload = true;
    state.files.loading = false;
    void closeFilesCursor();
  }
  $("#servers-page").hidden = page !== "servers";
  $("#workspace-page").hidden = page !== "workspace";
  $("#settings-page").hidden = page !== "settings";
  $("#nav-servers").classList.toggle("is-active", page === "servers");
  $("#nav-workspace").classList.toggle("is-active", page === "workspace");
  $("#nav-settings").classList.toggle("is-active", page === "settings");
  $("#nav-servers").setAttribute("aria-current", page === "servers" ? "page" : "false");
  $("#nav-workspace").setAttribute("aria-current", page === "workspace" ? "page" : "false");
  $("#nav-settings").setAttribute("aria-current", page === "settings" ? "page" : "false");
  setText($("#breadcrumb-current"), page === "servers" ? "服务器" : page === "settings" ? "设置" : "工作区");
  if (page === "settings") setSettingsCategory("appearance");
  syncWorkspaceActivity();
}

function setSettingsCategory(category) {
  for (const button of $$("[data-settings-target]")) {
    const selected = button.dataset.settingsTarget === category;
    button.classList.toggle("is-active", selected);
    button.setAttribute("aria-current", selected ? "page" : "false");
  }
  for (const section of $$("[data-settings-section]")) {
    section.hidden = section.dataset.settingsSection !== category;
  }
}

function setWorkspaceMode(mode) {
  state.workspaceMode = mode;
  const terminal = mode === "terminal";
  const files = mode === "files";
  const monitor = mode === "monitor";
  $("#workspace-page").dataset.mode = mode;
  $("#terminal-view").hidden = !terminal;
  $("#files-view").hidden = !files && !terminal;
  $("#monitor-view").hidden = !monitor;
  $("#workspace-terminal-tab").classList.toggle("is-active", terminal);
  $("#workspace-files-tab").classList.toggle("is-active", files);
  $("#workspace-monitor-tab").classList.toggle("is-active", monitor);
  $("#workspace-terminal-tab").setAttribute("aria-selected", String(terminal));
  $("#workspace-files-tab").setAttribute("aria-selected", String(files));
  $("#workspace-monitor-tab").setAttribute("aria-selected", String(monitor));
  $("#workspace-footer-status").textContent = terminal
    ? "终端数据直接通过 SSH 通道传输。"
    : files ? "文件通过加密 SFTP 通道读写。" : "监控数据通过固定采集脚本读取，不执行页面输入的命令。";
  syncWorkspaceActivity();
  if (!files && !terminal) {
    if (state.files.cursorId || state.files.loading) {
      state.files.sequence += 1;
      state.files.needsReload = true;
      state.files.loading = false;
      void closeFilesCursor();
      renderFiles();
    }
    if (terminal) requestAnimationFrame(() => {
      const active = state.terminals.get(state.activeTerminalId);
      if (active) fitTerminal(active);
    });
  }
  if ((files || terminal) && state.activeWorkspace && state.files.connectionId === state.activeWorkspace.connectionId) {
    if ((state.files.needsReload || state.files.entries.length === 0) && !state.files.loading) void loadFiles();
    else renderFiles();
    void loadTransferHistory();
  }
  if (monitor) renderMonitor();
}

const MONITOR_HISTORY_METRICS = [
  ["cpu", "cpuUsage", "cpu"],
  ["memory", "memoryUsage", "memory"],
  ["disk", "diskUsage", "disk"],
  ["load", "loadOneMinute", "load"],
  ["networkReceive", "networkReceiveRate", "network"],
  ["networkTransmit", "networkTransmitRate", "network"],
];

function syncWorkspaceActivity() {
  const entry = state.activeWorkspace;
  const activeConnectionId = entry?.snapshot?.state === "ready" ? entry.connectionId : null;
  const monitorVisible = state.currentPage === "workspace"
    && (state.workspaceMode === "terminal" || state.workspaceMode === "monitor")
    && activeConnectionId != null;
  const activityKey = `${activeConnectionId ?? ""}:${monitorVisible}`;
  if (activityKey !== state.monitor.activityKey) {
    state.monitor.activityKey = activityKey;
    void invoke("workspace_set_activity", { activeConnectionId, monitorVisible }).catch((error) => {
      if (monitorVisible) {
        state.monitor.error = errorMessage(error);
        renderMonitor();
      }
    });
  }
  if (monitorVisible && state.monitor.pollingConnectionId !== activeConnectionId) {
    startMonitorPolling(activeConnectionId);
  } else if (!monitorVisible && state.monitor.pollingConnectionId) {
    stopMonitorPolling();
  }
}

function stopMonitorPolling() {
  state.monitor.sequence += 1;
  state.monitor.pollingConnectionId = null;
  state.monitor.lastHistoryAt = 0;
}

function startMonitorPolling(connectionId) {
  stopMonitorPolling();
  const sequence = state.monitor.sequence;
  state.monitor.pollingConnectionId = connectionId;
  state.monitor.snapshot = null;
  state.monitor.histories = {};
  state.monitor.error = null;
  renderMonitor();
  void pollMonitor(sequence, connectionId);
}

function updateServerSearch(value, source) {
  state.query = value;
  const other = source === "global" ? $("#server-search") : $("#global-search");
  if (other.value !== value) other.value = value;
  renderServers();
}

function monitorRequestIsCurrent(sequence, connectionId) {
  return sequence === state.monitor.sequence
    && state.monitor.pollingConnectionId === connectionId
    && state.currentPage === "workspace"
    && (state.workspaceMode === "terminal" || state.workspaceMode === "monitor");
}

async function pollMonitor(sequence, connectionId) {
  while (monitorRequestIsCurrent(sequence, connectionId)) {
    try {
      state.monitor.snapshot = await invoke("monitor_get_snapshot", { connectionId });
      state.monitor.error = null;
      renderMonitor();
      if (Date.now() - state.monitor.lastHistoryAt >= 5000) {
        await loadMonitorHistory(sequence, connectionId);
        state.monitor.lastHistoryAt = Date.now();
        if (monitorRequestIsCurrent(sequence, connectionId)) renderMonitor();
      }
    } catch (error) {
      if (monitorRequestIsCurrent(sequence, connectionId)) {
        state.monitor.error = errorMessage(error);
        renderMonitor();
      }
    }
    await delay(1000);
  }
}

async function loadMonitorHistory(sequence, connectionId) {
  const snapshot = state.monitor.snapshot;
  const entries = await Promise.all(MONITOR_HISTORY_METRICS.map(async ([key, metric, qualityKey]) => {
    const quality = snapshot?.[qualityKey]?.quality;
    if (!quality || quality.status === "unsupported") return [key, []];
    try {
      const page = await invoke("monitor_get_history", {
        connectionId,
        metric,
        fromMs: Date.now() - 120_000,
        toMs: Date.now(),
        limit: 120,
      });
      return [key, page.samples];
    } catch {
      return [key, state.monitor.histories[key] ?? []];
    }
  }));
  if (monitorRequestIsCurrent(sequence, connectionId)) {
    state.monitor.histories = { ...state.monitor.histories, ...Object.fromEntries(entries) };
  }
}

function renderMonitorQuality(id, quality) {
  const element = $(id);
  const status = quality?.status ?? "warmingUp";
  element.className = `monitor-quality status-${status}`;
  element.textContent = qualityLabel(status, state.settings.value.language);
  element.title = quality?.errorCode ?? "";
}

function renderMonitorSparkline(id, series, secondary = null) {
  const svg = $(id);
  svg.replaceChildren();
  for (const [samples, className] of [[series, ""], [secondary, "is-secondary"]]) {
    if (!samples?.length) continue;
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", sparklinePath(samples));
    if (className) path.setAttribute("class", className);
    svg.append(path);
  }
}

function monitorPercent(value) {
  return typeof value === "number" && Number.isFinite(value) ? formatPercent(value) : "—";
}

function renderQuickMeter(id, value) {
  const meter = $(id);
  const bounded = typeof value === "number" && Number.isFinite(value) ? Math.min(100, Math.max(0, value)) : 0;
  meter.style.width = `${bounded}%`;
}

function renderQuickCpuChart(id, samples) {
  const chart = $(id);
  chart.replaceChildren();
  const values = (Array.isArray(samples) ? samples : [])
    .map((sample) => sample?.value)
    .filter((value) => typeof value === "number" && Number.isFinite(value))
    .slice(-16);
  const offset = (16 - values.length) * 10;
  for (const [index, value] of values.entries()) {
    const height = Math.max(3, Math.min(22, Math.max(0, value) / 100 * 22));
    const bar = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    bar.setAttribute("x", String(offset + index * 10));
    bar.setAttribute("y", (24 - height).toFixed(2));
    bar.setAttribute("width", "7");
    bar.setAttribute("height", height.toFixed(2));
    bar.setAttribute("rx", "1.5");
    chart.append(bar);
  }
}

function renderMonitor() {
  const snapshot = state.monitor.snapshot;
  const histories = state.monitor.histories;
  const refreshing = state.monitor.refreshing;
  $("#monitor-refresh").disabled = refreshing || !state.activeWorkspace || state.activeWorkspace.snapshot?.state !== "ready";
  $("#monitor-refresh").innerHTML = refreshing ? "正在刷新…" : `${iconMarkup("refresh-cw")}立即刷新`;
  const latestSample = snapshot
    ? Object.values(snapshot).filter((value) => value && typeof value === "object" && value.quality?.sampledAtMs != null)
      .map((value) => value.quality.sampledAtMs).reduce((latest, time) => Math.max(latest, time), 0)
    : 0;
  setText($("#monitor-refresh-status"), state.monitor.error
    ? "采集暂不可用"
    : latestSample ? `更新于 ${new Date(latestSample).toLocaleTimeString()}` : "正在采样");

  const cpu = snapshot?.cpu;
  setText($("#monitor-cpu-value"), monitorPercent(cpu?.usagePercent));
  setText($("#quick-cpu-value"), monitorPercent(cpu?.usagePercent));
  renderQuickCpuChart("#quick-cpu-chart", histories.cpu);
  setText($("#monitor-cpu-detail"), `逻辑核心 ${cpu?.logicalCores ?? "—"}`);
  renderMonitorQuality("#monitor-cpu-quality", cpu?.quality);
  renderMonitorSparkline("#monitor-cpu-chart", histories.cpu);

  const memory = snapshot?.memory;
  setText($("#monitor-memory-value"), monitorPercent(memory?.usedPercent));
  setText($("#quick-memory-value"), `${formatBytes(memory?.usedBytes)} / ${formatBytes(memory?.totalBytes)}`);
  renderQuickMeter("#quick-memory-meter", memory?.usedPercent);
  setText($("#monitor-memory-detail"), `已用 ${formatBytes(memory?.usedBytes)} / ${formatBytes(memory?.totalBytes)} · 可用 ${formatBytes(memory?.availableBytes)}`);
  renderMonitorQuality("#monitor-memory-quality", memory?.quality);
  renderMonitorSparkline("#monitor-memory-chart", histories.memory);

  const disk = snapshot?.disk;
  setText($("#monitor-disk-value"), monitorPercent(disk?.usedPercent));
  setText($("#quick-disk-value"), monitorPercent(disk?.usedPercent));
  renderQuickMeter("#quick-disk-meter", disk?.usedPercent);
  setText($("#monitor-disk-detail"), `已用 ${formatBytes(disk?.usedBytes)} / ${formatBytes(disk?.totalBytes)} · ${disk?.mount ?? "主文件系统"}`);
  renderMonitorQuality("#monitor-disk-quality", disk?.quality);
  renderMonitorSparkline("#monitor-disk-chart", histories.disk);

  const network = snapshot?.network;
  setText($("#quick-network-rx"), formatMonitorRate(network?.receivedBytesPerSecond));
  setText($("#quick-network-tx"), formatMonitorRate(network?.transmittedBytesPerSecond));
  setText($("#monitor-network-rx"), formatMonitorRate(network?.receivedBytesPerSecond));
  setText($("#monitor-network-tx"), formatMonitorRate(network?.transmittedBytesPerSecond));
  const interfaces = network?.interfaces?.map((item) => item.name).join(" · ");
  setText($("#monitor-network-detail"), interfaces ? `接口 ${interfaces} · 合计不含环回` : "服务器视角；不含环回接口");
  renderMonitorQuality("#monitor-network-quality", network?.quality);
  renderMonitorSparkline("#monitor-network-chart", histories.networkReceive, histories.networkTransmit);

  const load = snapshot?.load;
  const formatLoad = (value) => typeof value === "number" && Number.isFinite(value) ? value.toFixed(2) : "—";
  setText($("#quick-load-value"), `${formatLoad(load?.oneMinute)}  ${formatLoad(load?.fiveMinutes)}  ${formatLoad(load?.fifteenMinutes)}`);
  setText($("#monitor-load-value"), formatLoad(load?.oneMinute));
  setText($("#monitor-load-detail"), `${formatLoad(load?.oneMinute)} / ${formatLoad(load?.fiveMinutes)} / ${formatLoad(load?.fifteenMinutes)} · 1 / 5 / 15 分钟`);
  renderMonitorQuality("#monitor-load-quality", load?.quality);
  renderMonitorSparkline("#monitor-load-chart", histories.load);

  const system = snapshot?.system;
  renderMonitorQuality("#monitor-system-quality", system?.quality);
  setText($("#monitor-system-host"), system?.hostname ?? "—");
  setText($("#monitor-system-os"), system?.os ?? "—");
  setText($("#monitor-system-kernel"), system?.kernel ?? "—");
  setText($("#monitor-system-architecture"), system?.architecture ?? "—");
  setText($("#monitor-system-uptime"), formatUptime(snapshot?.uptime?.seconds, state.settings.value.language));
}

async function refreshMonitor() {
  const connectionId = state.activeWorkspace?.connectionId;
  if (!connectionId || state.monitor.refreshing) return;
  state.monitor.refreshing = true;
  state.monitor.error = null;
  renderMonitor();
  try {
    state.monitor.snapshot = await invoke("monitor_refresh", { connectionId });
    state.monitor.lastHistoryAt = 0;
    renderMonitor();
  } catch (error) {
    state.monitor.error = errorMessage(error);
    toast(state.monitor.error, "error");
  } finally {
    state.monitor.refreshing = false;
    renderMonitor();
  }
}

async function closeFilesCursor() {
  const cursorId = state.files.cursorId;
  state.files.cursorId = null;
  if (!cursorId) return;
  try { await invoke("sftp_list_close", { cursorId }); } catch { /* cursor cleanup is also bounded on the backend */ }
}

async function loadFiles({ path = state.files.path, append = false } = {}) {
  const connectionId = state.activeWorkspace?.connectionId;
  if (!connectionId || state.activeWorkspace?.snapshot?.state !== "ready") return;
  if (append && (!state.files.cursorId || state.files.loading)) return;

  const sequence = append ? state.files.sequence : ++state.files.sequence;
  if (!append) {
    state.files.path = path;
    state.files.entries = [];
    state.files.selectedPath = null;
    state.files.loading = true;
    renderFiles();
    await closeFilesCursor();
    if (sequence !== state.files.sequence) return;
  }
  const cursorId = append ? state.files.cursorId : null;
  state.files.loading = true;
  renderFiles();

  try {
    const page = append
      ? await invoke("sftp_list_next", { cursorId })
      : await invoke("sftp_list_start", { connectionId, path });
    if (sequence !== state.files.sequence || connectionId !== state.activeWorkspace?.connectionId) {
      if (page.cursorId) void invoke("sftp_list_close", { cursorId: page.cursorId }).catch(() => {});
      return;
    }
    state.files.path = page.path;
    state.files.cursorId = page.cursorId;
    state.files.entries = append ? [...state.files.entries, ...page.entries] : page.entries;
    state.files.loading = false;
    state.files.needsReload = false;
    $("#files-path-input").value = page.path;
    renderFiles();
  } catch (error) {
    if (sequence !== state.files.sequence) return;
    state.files.cursorId = null;
    state.files.loading = false;
    state.files.entries = [];
    state.files.selectedPath = null;
    renderFiles(errorMessage(error));
  }
}

function navigateFiles(path) {
  if (state.files.loading) state.files.sequence += 1;
  $("#files-path-bar").classList.remove("is-editing");
  $(".files-path-input-wrap").hidden = true;
  $("#files-go").hidden = true;
  $("#files-edit-path").setAttribute("aria-expanded", "false");
  void loadFiles({ path, append: false });
}

function renderFileBreadcrumb(path) {
  const breadcrumb = $("#files-path-breadcrumb");
  breadcrumb.replaceChildren();
  if (!path) return;

  const absolute = path.startsWith("/");
  const root = absolute ? "/" : ".";
  const rootButton = document.createElement("button");
  rootButton.className = "files-breadcrumb-segment";
  rootButton.type = "button";
  rootButton.textContent = root;
  rootButton.dataset.remotePath = root;
  rootButton.setAttribute("aria-label", absolute ? "根目录" : "当前目录");
  breadcrumb.append(rootButton);

  let current = root;
  const segments = path.split("/").filter(Boolean);
  if (!absolute && path === ".") segments.length = 0;
  for (const [index, segment] of segments.entries()) {
    const separator = document.createElement("span");
    separator.className = "files-breadcrumb-separator";
    separator.setAttribute("aria-hidden", "true");
    separator.textContent = "/";
    breadcrumb.append(separator);
    current = absolute
      ? `${current === "/" ? "" : current}/${segment}`
      : `${current === "." ? "" : `${current}/`}${segment}`;
    const button = document.createElement("button");
    button.className = "files-breadcrumb-segment";
    button.type = "button";
    button.textContent = segment;
    button.dataset.remotePath = absolute ? `/${current.replace(/^\//, "")}` : current;
    if (index === segments.length - 1) button.setAttribute("aria-current", "location");
    breadcrumb.append(button);
  }
}

function fileTypeLabel(entry) {
  if (entry.isSymlink || entry.fileType === "symlink") return "符号链接";
  if (entry.fileType === "directory") return "文件夹";
  if (entry.fileType === "file") return "文件";
  return "其他";
}

function formatFileSize(value) {
  if (value === null || value === undefined) return "—";
  try {
    let size = BigInt(value);
    if (size < 1024n) return `${size} B`;
    const units = ["KB", "MB", "GB", "TB", "PB"];
    let unit = -1;
    let divisor = 1n;
    while (size >= divisor * 1024n && unit < units.length - 1) {
      divisor *= 1024n;
      unit += 1;
    }
    const whole = size / divisor;
    const tenths = (size % divisor) * 10n / divisor;
    return `${whole}.${tenths} ${units[unit]}`;
  } catch {
    return "—";
  }
}

function formatModifiedAt(value) {
  if (value === null || value === undefined) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return "—";
  const day = date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  const time = date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hourCycle: "h23" });
  return `${day} ${time}`;
}

function renderFiles(error = null) {
  const pathInput = $("#files-path-input");
  if (document.activeElement !== pathInput && state.files.path) pathInput.value = state.files.path;
  renderFileBreadcrumb(state.files.path);
  const profile = state.activeWorkspace?.profile;
  $("#files-view-meta").textContent = profile ? `${profile.name || profile.host} · SFTP · ${state.files.path || "—"}` : "";
  $("#files-path-status").textContent = state.files.loading ? "正在读取…" : error ? "读取失败" : `${state.files.entries.length} 项已加载`;
  const workspaceUnavailable = !state.activeWorkspace || state.activeWorkspace.snapshot?.state !== "ready";
  $("#files-parent").disabled = workspaceUnavailable || state.disconnecting || state.files.loading || !state.files.path || state.files.path === "/";
  $("#files-go").disabled = workspaceUnavailable || state.disconnecting || state.files.loading;
  $("#files-refresh").disabled = workspaceUnavailable || state.disconnecting || state.files.loading;
  $("#files-path-input").disabled = workspaceUnavailable || state.disconnecting;
  $("#files-edit-path").disabled = workspaceUnavailable || state.disconnecting || state.files.loading;
  $("#files-load-more").hidden = !state.files.cursorId;
  $("#files-load-more").disabled = workspaceUnavailable || state.disconnecting || state.files.loading;

  const selected = state.files.entries.find((entry) => entry.path === state.files.selectedPath) ?? null;
  const hasSelection = Boolean(selected);
  const listBusy = state.disconnecting || state.files.loading || state.files.needsReload;
  $("#files-selection-label").textContent = selected ? `${selected.name} · ${fileTypeLabel(selected)}` : "选择文件以查看操作";
  $("#files-upload").disabled = workspaceUnavailable || listBusy || !state.files.path;
  $("#files-new-folder").disabled = workspaceUnavailable || listBusy || !state.files.path;
  $("#files-copy-path").disabled = workspaceUnavailable || listBusy || !hasSelection;
  $("#files-download").disabled = workspaceUnavailable || listBusy || !selected || !["file", "symlink"].includes(selected.fileType);
  $("#files-rename").disabled = workspaceUnavailable || listBusy || !selected;
  $("#files-delete").disabled = workspaceUnavailable || listBusy || !selected;
  $("#files-entry-count").textContent = state.files.loading
    ? "正在读取远程目录…"
    : error
      ? error
      : `${state.files.entries.length} 个项目${state.files.cursorId ? " · 还有更多" : ""}`;

  const list = $("#files-list");
  list.replaceChildren();
  if (error || (!state.files.entries.length && !state.files.loading)) {
    const empty = document.createElement("div");
    empty.className = "files-empty";
    const title = document.createElement("strong");
    title.textContent = state.files.loading ? "正在读取目录" : error ? "无法读取目录" : "此目录为空";
    const detail = document.createElement("span");
    detail.textContent = error ?? (state.files.loading ? "正在连接远程 SFTP…" : "可以上传文件或新建文件夹。");
    empty.append(title, detail);
    list.append(empty);
    renderTransfers();
    return;
  }

  for (const entry of state.files.entries) {
    const row = document.createElement("div");
    row.className = `file-row${entry.path === state.files.selectedPath ? " is-selected" : ""}`;
    row.setAttribute("role", "row");
    row.setAttribute("aria-selected", String(entry.path === state.files.selectedPath));
    row.tabIndex = 0;
    row.dataset.filePath = entry.path;
    row.title = entry.path;
    const nameCell = document.createElement("span");
    nameCell.className = "file-name-cell";
    nameCell.setAttribute("role", "cell");
    const glyph = document.createElement("span");
    const isLink = entry.isSymlink || entry.fileType === "symlink";
    glyph.className = `file-glyph${entry.fileType === "directory" ? " is-folder" : isLink ? " is-link" : ""}`;
    glyph.setAttribute("aria-hidden", "true");
    glyph.innerHTML = iconMarkup(entry.fileType === "directory" ? "folder" : isLink ? "arrow-up-right" : "file");
    const name = document.createElement("span");
    name.className = "file-name-text";
    name.dataset.userContent = "true";
    name.textContent = entry.name;
    nameCell.append(glyph, name);
    const type = document.createElement("span");
    type.className = "file-type-cell";
    type.setAttribute("role", "cell");
    type.textContent = fileTypeLabel(entry);
    const size = document.createElement("span");
    size.className = "file-size-cell";
    size.setAttribute("role", "cell");
    size.textContent = entry.fileType === "directory" ? "—" : formatFileSize(entry.sizeBytes);
    const date = document.createElement("span");
    date.className = "file-date-cell";
    date.setAttribute("role", "cell");
    date.textContent = formatModifiedAt(entry.modifiedAtMs);
    row.append(nameCell, type, size, date);
    const menuButton = document.createElement("button");
    menuButton.className = "file-row-menu";
    menuButton.type = "button";
    menuButton.setAttribute("aria-label", `${entry.name} · ${translateText("更多操作")}`);
    menuButton.setAttribute("aria-haspopup", "menu");
    menuButton.setAttribute("aria-expanded", "false");
    menuButton.innerHTML = iconMarkup("ellipsis");
    menuButton.addEventListener("click", (event) => {
      event.stopPropagation();
      state.files.selectedPath = entry.path;
      renderFiles();
      const trigger = $$(".file-row").find((item) => item.dataset.filePath === entry.path)?.querySelector(".file-row-menu");
      if (!trigger) return;
      trigger.setAttribute("aria-expanded", "true");
      showFileActionMenu(entry, event.clientX, event.clientY, trigger);
    });
    row.append(menuButton);
    row.addEventListener("click", (event) => {
      const wasSelected = entry.path === state.files.selectedPath;
      state.files.selectedPath = entry.path;
      renderFiles();
      if (event.detail === 0 && wasSelected) void openRemoteEntry(entry);
    });
    row.addEventListener("dblclick", () => void openRemoteEntry(entry));
    row.addEventListener("keydown", (event) => {
      if (event.target !== row || (event.key !== "Enter" && event.key !== " ")) return;
      event.preventDefault();
      if (entry.path === state.files.selectedPath) void openRemoteEntry(entry);
      else {
        state.files.selectedPath = entry.path;
        renderFiles();
        $$(".file-row").find((item) => item.dataset.filePath === entry.path)?.focus();
      }
    });
    row.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      if (entry.path !== state.files.selectedPath) {
        state.files.selectedPath = entry.path;
        renderFiles();
      }
      const menuTrigger = $$(".file-row").find((item) => item.dataset.filePath === entry.path)?.querySelector(".file-row-menu");
      if (!menuTrigger) return;
      menuTrigger.setAttribute("aria-expanded", "true");
      showFileActionMenu(entry, event.clientX, event.clientY, menuTrigger);
    });
    list.append(row);
  }
  renderTransfers();
}

async function openRemoteEntry(entry) {
  if (entry.fileType === "directory") {
    navigateFiles(entry.path);
    return;
  }
  if (entry.fileType === "symlink" || entry.isSymlink) {
    try {
      const target = await invoke("sftp_stat", {
        connectionId: state.activeWorkspace.connectionId,
        path: entry.path,
        followSymlink: true,
      });
      if (target.fileType === "directory") navigateFiles(entry.path);
      else {
        state.files.selectedPath = entry.path;
        renderFiles();
        toast("已选择文件。使用“下载”保存到本机。", "success");
      }
    } catch (error) {
      toast(errorMessage(error), "error");
    }
    return;
  }
  state.files.selectedPath = entry.path;
  renderFiles();
  toast("已选择文件。使用“下载”保存到本机。", "success");
}

async function loadTransferHistory() {
  const connectionId = state.activeWorkspace?.connectionId;
  if (!connectionId || state.workspaceMode !== "files") return;
  try {
    const transfers = await invoke("sftp_transfer_list", { connectionId, limit: 10 });
    if (connectionId !== state.activeWorkspace?.connectionId) return;
    state.files.transfers.clear();
    state.files.transferOrder = [];
    for (const transfer of transfers) rememberTransfer(transfer);
    renderTransfers();
  } catch (error) {
    toast(`读取传输任务失败：${errorMessage(error)}`, "error");
  }
}

function rememberTransfer(snapshot) {
  if (state.files.dismissedTransferIds.has(snapshot.transferId) && snapshot.state === "completed") return;
  if (!state.files.transfers.has(snapshot.transferId)) state.files.transferOrder.unshift(snapshot.transferId);
  state.files.transfers.set(snapshot.transferId, snapshot);
  state.files.transferOrder = state.files.transferOrder.slice(0, 10);
  for (const id of [...state.files.transfers.keys()]) {
    if (!state.files.transferOrder.includes(id)) state.files.transfers.delete(id);
  }
}

function transferStatus(snapshot) {
  if (snapshot.state === "created") return "准备中";
  if (snapshot.state === "transferring") return snapshot.direction === "upload" ? "正在上传" : "正在下载";
  if (snapshot.state === "finalizing") return "正在发布";
  if (snapshot.state === "completed") return "已完成";
  if (snapshot.state === "cancelled") return "已取消";
  if (snapshot.error?.messageKey) return knownErrors[snapshot.error.messageKey] ?? "失败";
  return "失败";
}

function formatRate(value) {
  if (!Number.isFinite(value) || value <= 0) return "—";
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let rate = value;
  let unit = 0;
  while (rate >= 1024 && unit < units.length - 1) { rate /= 1024; unit += 1; }
  return `${rate < 10 ? rate.toFixed(1) : Math.round(rate)} ${units[unit]}`;
}

function transferProgress(snapshot) {
  try {
    if (snapshot.state === "completed") return { percent: 100, label: "100%" };
    const transferred = BigInt(snapshot.transferredBytes ?? "0");
    const total = snapshot.totalBytes == null ? 0n : BigInt(snapshot.totalBytes);
    if (total <= 0n) return { percent: 0, label: formatFileSize(transferred.toString()) };
    const tenths = Number(transferred * 1000n / total) / 10;
    return { percent: Math.min(100, Math.max(0, tenths)), label: `${tenths.toFixed(1)}%` };
  } catch {
    return { percent: 0, label: "—" };
  }
}

function renderTransfers() {
  const list = $("#transfer-list");
  if (!list) return;
  list.replaceChildren();
  const transfers = state.files.transferOrder
    .map((id) => state.files.transfers.get(id))
    .filter(Boolean);
  const active = transfers.filter((transfer) => !["completed", "cancelled", "failed"].includes(transfer.state)).length;
  $("#transfer-count").textContent = String(active);
  $("#files-clear-completed").hidden = !transfers.some((transfer) => transfer.state === "completed");
  if (!transfers.length) {
    const empty = document.createElement("div");
    empty.className = "transfer-empty";
    empty.textContent = "上传或下载文件后，任务进度会显示在这里。";
    list.append(empty);
    return;
  }
  for (const transfer of transfers) {
    const row = document.createElement("div");
    row.className = "transfer-row";
    const file = document.createElement("div");
    file.className = "transfer-file";
    const direction = document.createElement("span");
    direction.className = "transfer-direction";
    direction.innerHTML = iconMarkup(transfer.direction === "upload" ? "upload" : "download");
    const name = document.createElement("span");
    name.className = "transfer-file-name";
    name.dataset.userContent = "true";
    name.textContent = transfer.fileName;
    name.title = transfer.fileName;
    file.append(direction, name);
    const progressWrap = document.createElement("div");
    progressWrap.className = "transfer-progress-wrap";
    const progress = transferProgress(transfer);
    const bar = document.createElement("div");
    bar.className = "transfer-progress";
    const fill = document.createElement("span");
    fill.style.width = `${progress.percent}%`;
    bar.append(fill);
    const percent = document.createElement("span");
    percent.className = "transfer-progress-label";
    percent.textContent = progress.label;
    progressWrap.append(bar, percent);
    const meta = document.createElement("span");
    meta.className = "transfer-meta";
    const rate = formatRate(transfer.bytesPerSecond);
    meta.textContent = [
      rate === "—" ? "" : rate,
      transfer.remainingSeconds == null ? "" : `剩余 ${transfer.remainingSeconds}s`,
    ].filter(Boolean).join(" · ");
    meta.hidden = !meta.textContent;
    const status = document.createElement("span");
    status.className = `transfer-status${transfer.state === "failed" ? " is-failed" : ""}`;
    status.textContent = transferStatus(transfer);
    row.append(file, progressWrap, meta, status);
    if (!["completed", "cancelled", "failed"].includes(transfer.state)) {
      const cancel = document.createElement("button");
      cancel.className = "transfer-cancel";
      cancel.type = "button";
      cancel.textContent = "取消";
      cancel.addEventListener("click", () => void cancelTransfer(transfer.transferId));
      row.append(cancel);
    }
    if (transfer.cleanupRequired && transfer.temporaryPath) {
      const cleanup = document.createElement("div");
      cleanup.className = "transfer-meta transfer-cleanup";
      cleanup.textContent = `需要清理临时文件：${transfer.temporaryPath}`;
      cleanup.title = transfer.temporaryPath;
      row.append(cleanup);
    }
    list.append(row);
  }
}

function startTransfer(command, payload) {
  if (state.disconnecting) return;
  const channel = new core.Channel();
  channel.onmessage = (snapshot) => {
    rememberTransfer(snapshot);
    renderTransfers();
    if (snapshot.state === "completed" && snapshot.direction === "upload") void loadFiles({ path: state.files.path });
  };
  const pending = invoke(command, payload, { outputChannel: channel });
  state.transferStarts.add(pending);
  void pending
    .then((snapshot) => {
      rememberTransfer(snapshot);
      renderTransfers();
    })
    .catch((error) => {
      channel.onmessage = null;
      toast(errorMessage(error), "error", 6500);
    })
    .finally(() => state.transferStarts.delete(pending));
}

async function cancelTransfer(transferId) {
  try {
    const snapshot = await invoke("sftp_transfer_cancel", { transferId });
    rememberTransfer(snapshot);
    renderTransfers();
  } catch (error) {
    toast(`取消传输失败：${errorMessage(error)}`, "error");
  }
}

async function selectUpload() {
  try {
    if (state.disconnecting || state.files.loading || state.files.needsReload || !state.activeWorkspace || !state.files.path) return toast("请先等待远程目录加载完成。", "error");
    const selected = await invoke("local_file_select", { purpose: "upload" });
    if (!selected || state.disconnecting) return;
    startTransfer("sftp_upload", {
      connectionId: state.activeWorkspace.connectionId,
      localFileToken: selected.token,
      remotePath: joinRemotePath(state.files.path, selected.displayName),
    });
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

async function selectDownload() {
  if (state.disconnecting || state.files.loading || state.files.needsReload) return;
  const selected = state.files.entries.find((entry) => entry.path === state.files.selectedPath);
  if (!selected) return;
  try {
    const stat = await invoke("sftp_stat", {
      connectionId: state.activeWorkspace.connectionId,
      path: selected.path,
      followSymlink: true,
    });
    if (stat.fileType !== "file") throw { code: "VALIDATION_FAILED", messageKey: "errors.sftpTransferFileTypeUnsupported" };
    const destination = await invoke("local_file_select", { purpose: "download" });
    if (!destination || state.disconnecting) return;
    startTransfer("sftp_download", {
      connectionId: state.activeWorkspace.connectionId,
      remotePath: selected.path,
      localFileToken: destination.token,
    });
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

function openFileNameDialog(action) {
  state.files.action = action;
  const rename = action === "rename";
  const selected = state.files.entries.find((entry) => entry.path === state.files.selectedPath);
  $("#file-name-dialog-title").textContent = rename ? "重命名项目" : "新建文件夹";
  $("#file-name-dialog-description").textContent = rename ? "输入新的名称；项目会保留在当前目录。" : "在当前远程目录中创建一个文件夹。";
  $("#file-name-label").textContent = rename ? "新名称" : "文件夹名称";
  $("#file-name-submit").textContent = rename ? "重命名" : "创建文件夹";
  $("#file-name-input").value = rename ? selected?.name ?? "" : "";
  $("#file-name-error").hidden = true;
  openDialog($("#file-name-dialog"));
  window.setTimeout(() => $("#file-name-input").focus(), 20);
  if (rename) $("#file-name-input").select();
}

async function submitFileName(event) {
  event.preventDefault();
  const value = $("#file-name-input").value.trim();
  if (!value || value === "." || value === ".." || /[\\/\0]/.test(value)) {
    $("#file-name-error").textContent = "请输入不含斜线的有效名称。";
    $("#file-name-error").hidden = false;
    return;
  }
  const action = state.files.action;
  const entry = state.files.entries.find((item) => item.path === state.files.selectedPath);
  const connectionId = state.activeWorkspace?.connectionId;
  const submit = $("#file-name-submit");
  submit.disabled = true;
  try {
    if (action === "rename" && entry) {
      await invoke("sftp_rename", { connectionId, sourcePath: entry.path, newName: value });
      toast("项目已重命名。");
    } else {
      await invoke("sftp_mkdir", { connectionId, parentPath: state.files.path, name: value });
      toast("文件夹已创建。");
    }
    closeDialog($("#file-name-dialog"));
    await loadFiles({ path: state.files.path });
  } catch (error) {
    $("#file-name-error").textContent = errorMessage(error);
    $("#file-name-error").hidden = false;
  } finally {
    submit.disabled = false;
  }
}

function openFileDeleteDialog() {
  const entry = state.files.entries.find((item) => item.path === state.files.selectedPath);
  if (!entry) return;
  $("#file-delete-title").textContent = entry.fileType === "directory" ? "删除远程文件夹？" : "删除远程文件？";
  $("#file-delete-description").textContent = entry.fileType === "directory"
    ? `将删除空文件夹“${entry.name}”。非空文件夹会被保留。`
    : entry.fileType === "symlink" || entry.isSymlink
      ? `将删除符号链接“${entry.name}”，不会删除它指向的文件。`
      : `将永久删除远程文件“${entry.name}”。`;
  openDialog($("#file-delete-dialog"));
}

async function deleteSelectedFile() {
  const entry = state.files.entries.find((item) => item.path === state.files.selectedPath);
  if (!entry || !state.activeWorkspace) return;
  $("#file-delete-confirm").disabled = true;
  try {
    await invoke("sftp_delete", {
      connectionId: state.activeWorkspace.connectionId,
      path: entry.path,
      expectedType: entry.isSymlink ? "symlink" : entry.fileType,
      confirmed: true,
    });
    closeDialog($("#file-delete-dialog"));
    toast("远程项目已删除。");
    await loadFiles({ path: state.files.path });
  } catch (error) {
    toast(errorMessage(error), "error");
  } finally {
    $("#file-delete-confirm").disabled = false;
  }
}

async function copySelectedPath() {
  const entry = state.files.entries.find((item) => item.path === state.files.selectedPath);
  if (!entry) return;
  try {
    await navigator.clipboard.writeText(entry.path);
    toast("远程路径已复制。");
  } catch {
    toast("无法访问剪贴板，请手动复制路径。", "error");
  }
}

function enterWorkspace(entry) {
  if (!entry || entry.snapshot?.state !== "ready") return;
  const connectionChanged = state.files.connectionId !== entry.connectionId;
  if (connectionChanged) {
    state.files.sequence += 1;
    void closeFilesCursor();
    state.files.connectionId = entry.connectionId;
    state.files.path = ".";
    state.files.entries = [];
    state.files.selectedPath = null;
    state.files.loading = false;
    state.files.needsReload = true;
    state.files.transfers.clear();
    state.files.transferOrder = [];
    state.files.dismissedTransferIds.clear();
    state.workspaceMode = "terminal";
  }
  state.activeWorkspace = entry;
  if ($(".app-shell").classList.contains("is-focused")) setTerminalFocus(false);
  $("#workspace-server-name").dataset.userContent = entry.profile?.name || entry.profile?.host ? "true" : "false";
  $("#workspace-server-address").dataset.userContent = "true";
  $("#workspace-server-monogram").dataset.userContent = "true";
  setText($("#workspace-server-name"), entry.profile?.name ?? entry.profile?.host ?? "服务器工作区");
  setText($("#workspace-server-address"), `${entry.profile?.username ?? ""}@${entry.profile?.host ?? ""}:${entry.profile?.port ?? 22}`);
  setText($("#workspace-server-monogram"), (entry.profile?.name ?? entry.profile?.host ?? "S").slice(0, 1).toUpperCase());
  setText($("#detail-host"), entry.profile?.host ?? "—");
  setText($("#detail-port"), entry.profile?.port ?? 22);
  setText($("#detail-user"), entry.profile?.username ?? "—");
  setText($("#detail-auth"), entry.profile?.authType === "privateKey" ? "SSH 私钥" : "密码");
  $("#nav-workspace").disabled = false;
  $("#nav-live-dot").hidden = false;
  showPage("workspace");
  setWorkspaceMode(state.workspaceMode);
  renderTerminalTabs();
  if (state.terminals.size === 0) void openTerminal();
  else focusTerminalInput();
}

function createTerminalView(label) {
  const Terminal = window.Terminal;
  const FitAddon = window.FitAddon?.FitAddon;
  if (!Terminal || !FitAddon) throw new Error("本地交互式终端组件未能加载。");

  const mount = document.createElement("div");
  mount.className = "terminal-mount";
  $("#terminal-stack").append(mount);

  const instance = new Terminal({
    allowProposedApi: false,
    cursorBlink: true,
    cursorStyle: state.settings.value.terminalCursorStyle,
    fontFamily: state.settings.value.terminalFontFamily,
    fontSize: state.settings.value.terminalFontSize,
    lineHeight: 1.35,
    scrollback: state.settings.value.terminalScrollbackLines,
    screenReaderMode: true,
    theme: {
      background: "#000000",
      foreground: "#a7b0be",
      cursor: "#5b5ce2",
      cursorAccent: "#000000",
      selectionBackground: "#373785",
      black: "#080808",
      red: "#e06c75",
      green: "#22a06b",
      yellow: "#e9a23b",
      blue: "#5b8def",
      magenta: "#b084f5",
      cyan: "#56b6c2",
      white: "#d0d5dd",
      brightBlack: "#667085",
      brightRed: "#f08d7e",
      brightGreen: "#3cc287",
      brightYellow: "#f3bf65",
      brightBlue: "#84a9ff",
      brightMagenta: "#c3a0ff",
      brightCyan: "#73cbd6",
      brightWhite: "#f2f4f7",
    },
  });
  const fitAddon = new FitAddon();
  instance.loadAddon(fitAddon);
  instance.open(mount);

  const entry = {
    terminalId: null,
    streamId: null,
    channel: new core.Channel(),
    mount,
    instance,
    fitAddon,
    waitingChunks: [],
    ready: false,
    watching: true,
    state: "opening",
    label,
    outputFailed: false,
    inputFailed: false,
    pendingInputBytes: 0,
    inputQueue: Promise.resolve(),
    inputSequence: 1,
    outputQueue: null,
    outputPending: Promise.resolve(),
    resizeQueue: Promise.resolve(),
    resizeTimer: null,
    pendingSize: null,
    lastSentSize: null,
  };
  entry.outputQueue = createTerminalOutputConsumer(
    (bytes, callback) => entry.instance.write(bytes, callback),
    (chunk) => invoke("terminal_ack", {
      terminalId: entry.terminalId,
      streamId: entry.streamId,
      seq: chunk.seq,
    }),
  );
  instance.onData((data) => sendTerminalText(entry, data));
  instance.onSelectionChange(() => {
    window.clearTimeout(entry.selectionCopyTimer);
    if (!isTerminalCopyOnSelectEnabled(localStorage)) return;
    entry.selectionCopyTimer = window.setTimeout(() => {
      if (!isTerminalCopyOnSelectEnabled(localStorage)) return;
      const selection = entry.instance.getSelection();
      void copyTerminalSelection(selection, true, navigator.clipboard).catch(() => {
        if (!terminalClipboardWarningShown) {
          terminalClipboardWarningShown = true;
          toast("无法访问剪贴板，请使用键盘快捷键复制终端内容。", "error");
        }
      });
    }, 120);
  });
  instance.onResize(({ cols, rows }) => {
    if (state.activeTerminalId === entry.terminalId || !entry.terminalId) {
      setText($("#terminal-size"), `${cols} × ${rows}`);
    }
    scheduleTerminalResize(entry, { columns: cols, rows });
  });
  fitAddon.fit();
  return entry;
}

function handleTerminalChunk(entry, chunk) {
  if (chunk.terminalId !== entry.terminalId || chunk.streamId !== entry.streamId) {
    failTerminalOutput(entry, new Error("终端输出来自无效的数据流。"));
    return;
  }
  entry.outputPending = entry.outputQueue(chunk);
  void entry.outputPending.catch((error) => failTerminalOutput(entry, error));
}

function failTerminalOutput(entry, error) {
  if (entry.outputFailed) return;
  entry.outputFailed = true;
  entry.watching = false;
  entry.state = "failed";
  entry.channel.onmessage = null;
  entry.instance.write(`\r\n\u001b[31m[终端输出已停止：${errorMessage(error)}]\u001b[0m\r\n`);
  setText($("#terminal-input-state"), "输出已停止");
  renderTerminalTabs();
  toast(`终端输出已停止：${errorMessage(error)}`, "error", 7000);
  if (entry.terminalId) void invoke("terminal_close", { terminalId: entry.terminalId }).catch(() => {});
}

function scheduleTerminalResize(entry, dimensions) {
  entry.pendingSize = dimensions;
  if (!entry.terminalId || entry.state !== "running") return;
  window.clearTimeout(entry.resizeTimer);
  entry.resizeTimer = window.setTimeout(() => {
    const size = entry.pendingSize;
    if (!size || (size.columns === entry.lastSentSize?.columns && size.rows === entry.lastSentSize?.rows)) return;
    const pixelWidth = Math.max(1, Math.round(entry.mount.clientWidth));
    const pixelHeight = Math.max(1, Math.round(entry.mount.clientHeight));
    entry.resizeQueue = entry.resizeQueue
      .then(() => invoke("terminal_resize", { terminalId: entry.terminalId, ...size, pixelWidth, pixelHeight }))
      .then((result) => { entry.lastSentSize = { columns: result.columns, rows: result.rows }; })
      .catch((error) => toast(`终端尺寸更新失败：${errorMessage(error)}`, "error", 7000));
  }, 80);
}

function fitTerminal(entry) {
  if (!entry || !entry.mount.isConnected) return;
  entry.fitAddon.fit();
}

function renderTerminalTabs() {
  const tabs = $("#terminal-tabs");
  tabs.replaceChildren();
  for (const terminalId of state.terminalOrder) {
    const entry = state.terminals.get(terminalId);
    if (!entry) continue;
    const tab = document.createElement("button");
    tab.className = `terminal-tab${state.activeTerminalId === terminalId ? " is-active" : ""}`;
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-selected", String(state.activeTerminalId === terminalId));
    const label = document.createElement("span");
    label.textContent = entry.label;
    const close = document.createElement("span");
    close.className = "terminal-tab-close";
    close.innerHTML = iconMarkup("x");
    close.title = "关闭终端";
    tab.append(label, close);
    tab.addEventListener("click", (event) => {
      if (event.target === close) {
        event.stopPropagation();
        void closeTerminal(terminalId);
        return;
      }
      activateTerminal(terminalId);
    });
    tabs.append(tab);
  }
  for (const [terminalId, entry] of state.terminals) {
    entry.mount.classList.toggle("is-active", terminalId === state.activeTerminalId);
  }
  const active = state.terminals.get(state.activeTerminalId);
  $("#terminal-empty").hidden = Boolean(active);
  $("#clear-terminal").disabled = !active;
  setText($("#terminal-title"), active?.label ?? "交互式终端");
  setText($("#terminal-input-state"), active?.inputFailed
    ? "输入暂停，请重新连接"
    : active?.state === "running"
      ? "Shell 就绪"
      : active?.state === "opening"
        ? "正在启动"
        : active
          ? "会话已停止"
          : "等待 Shell");
  setText($("#terminal-size"), active ? `${active.instance.cols} × ${active.instance.rows}` : "— × —");
  setText($("#active-terminal-count"), `${state.terminals.size} 个终端`);
}

function activateTerminal(terminalId) {
  const entry = state.terminals.get(terminalId);
  if (!entry) return;
  state.activeTerminalId = terminalId;
  renderTerminalTabs();
  requestAnimationFrame(() => {
    fitTerminal(entry);
    focusTerminalInput();
  });
}

function focusTerminalInput() {
  window.setTimeout(() => state.terminals.get(state.activeTerminalId)?.instance.focus(), 20);
}

async function openTerminal() {
  const workspace = state.activeWorkspace;
  if (!workspace || workspace.snapshot?.state !== "ready" || !core?.Channel) return;
  let terminal;
  try {
    terminal = createTerminalView(`终端 ${state.nextTerminalNumber++}`);
  } catch (error) {
    toast(`无法启动交互式终端：${errorMessage(error)}`, "error", 7000);
    return;
  }
  terminal.channel.onmessage = (chunk) => {
    if (!terminal.ready) terminal.waitingChunks.push(chunk);
    else handleTerminalChunk(terminal, chunk);
  };
  if (state.terminals.size === 0) {
    $("#terminal-empty").hidden = false;
    setText($("#terminal-input-state"), "正在启动");
  }
  try {
    const opened = await invoke("terminal_open", {
      connectionId: workspace.connectionId,
      columns: terminal.instance.cols,
      rows: terminal.instance.rows,
      pixelWidth: Math.max(1, Math.round(terminal.mount.clientWidth)),
      pixelHeight: Math.max(1, Math.round(terminal.mount.clientHeight)),
    }, { outputChannel: terminal.channel });
    terminal.terminalId = opened.terminalId;
    terminal.streamId = opened.streamId;
    terminal.ready = true;
    terminal.state = "running";
    terminal.lastSentSize = { columns: terminal.instance.cols, rows: terminal.instance.rows };
    state.terminals.set(terminal.terminalId, terminal);
    state.terminalOrder.push(terminal.terminalId);
    state.activeTerminalId = terminal.terminalId;
    renderTerminalTabs();
    for (const chunk of terminal.waitingChunks.splice(0)) handleTerminalChunk(terminal, chunk);
    void watchTerminal(terminal);
    focusTerminalInput();
  } catch (error) {
    terminal.channel.onmessage = null;
    terminal.instance.dispose();
    terminal.mount.remove();
    renderTerminalTabs();
    toast(`SSH 已连接，但无法打开 Terminal：${errorMessage(error)}`, "error", 7000);
  }
}

async function watchTerminal(terminal) {
  while (terminal.watching && terminal.state === "running") {
    await delay(1200);
    try {
      const snapshot = await invoke("terminal_get", { terminalId: terminal.terminalId });
      terminal.state = snapshot.state;
      if (["closed", "failed"].includes(snapshot.state)) {
        terminal.watching = false;
        await terminal.outputPending.catch(() => {});
        const ending = snapshot.state === "failed"
          ? translateText(`[终端已停止：${errorMessage(snapshot.error ?? "连接错误")}]`)
          : `[${translateText("远程 Shell 已结束")}]`;
        terminal.instance.write(`\r\n\u001b[90m${ending}\u001b[0m\r\n`);
        renderTerminalTabs();
        if (snapshot.state === "failed") toast(errorMessage(snapshot.error ?? "终端已停止。"), "error", 7000);
        return;
      }
    } catch (error) {
      terminal.watching = false;
      terminal.state = "failed";
      terminal.instance.write(`\r\n\u001b[31m${translateText(`[无法读取终端状态：${errorMessage(error)}]`)}\u001b[0m\r\n`);
      renderTerminalTabs();
      return;
    }
  }
}

function sendTerminalText(terminal, text) {
  if (terminal.state !== "running" || terminal.inputFailed) return;
  const bytes = new TextEncoder().encode(text);
  if (terminal.pendingInputBytes + bytes.byteLength > MAX_QUEUED_TERMINAL_INPUT) {
    toast("终端输入暂存已满，请稍候再粘贴。", "error");
    return;
  }
  terminal.pendingInputBytes += bytes.byteLength;
  let acceptedBytes = 0;
  terminal.inputQueue = terminal.inputQueue
    .then(async () => {
      for (let offset = 0; offset < bytes.byteLength; offset += MAX_TERMINAL_INPUT_CHUNK) {
        if (terminal.inputFailed || terminal.state !== "running") return;
        const chunk = bytes.subarray(offset, offset + MAX_TERMINAL_INPUT_CHUNK);
        const sequence = terminal.inputSequence;
        await invoke("terminal_write", {
          terminalId: terminal.terminalId,
          inputSeq: String(sequence),
          dataBase64: encodeBytesBase64(chunk),
        });
        terminal.inputSequence = sequence + 1;
        acceptedBytes += chunk.byteLength;
      }
    })
    .catch((error) => {
      terminal.inputFailed = true;
      terminal.instance.options.disableStdin = true;
      setText($("#terminal-input-state"), "输入发送失败，请重新连接");
      const partial = acceptedBytes > 0 ? "当前输入只发送了一部分。" : "当前输入未发送。";
      toast(`终端输入发送失败：${errorMessage(error)}。${partial}请关闭并重新打开终端。`, "error", 7000);
    })
    .finally(() => { terminal.pendingInputBytes -= bytes.byteLength; });
}

async function closeTerminal(terminalId) {
  const entry = state.terminals.get(terminalId);
  if (!entry) return;
  entry.state = "closing";
  entry.watching = false;
  window.clearTimeout(entry.resizeTimer);
  window.clearTimeout(entry.selectionCopyTimer);
  entry.channel.onmessage = null;
  await entry.resizeQueue;
  try {
    await invoke("terminal_close", { terminalId });
  } catch (error) {
    toast(`关闭终端失败：${errorMessage(error)}`, "error");
  }
  entry.instance.dispose();
  entry.mount.remove();
  state.terminals.delete(terminalId);
  state.terminalOrder = state.terminalOrder.filter((id) => id !== terminalId);
  if (state.activeTerminalId === terminalId) state.activeTerminalId = state.terminalOrder.at(-1) ?? null;
  renderTerminalTabs();
  const active = state.terminals.get(state.activeTerminalId);
  if (active) requestAnimationFrame(() => { fitTerminal(active); focusTerminalInput(); });
}

async function closeAllTerminals() {
  const terminalIds = [...state.terminalOrder];
  for (const terminalId of terminalIds) await closeTerminal(terminalId);
}

async function disconnectWorkspace() {
  const entry = state.activeWorkspace;
  if (!entry || state.disconnecting) return;
  if (state.settings.value.confirmBeforeDisconnect
    && !window.confirm(`确认断开与“${entry.profile?.name ?? entry.profile?.host ?? "服务器"}”的连接吗？`)) return;
  state.disconnecting = true;
  $("#disconnect-server").disabled = true;
  renderFiles();
  try {
    await Promise.allSettled([...state.transferStarts]);
    const transfers = await invoke("sftp_transfer_list", { connectionId: entry.connectionId, limit: 10 });
    const activeTransfers = transfers.filter((transfer) => !["completed", "cancelled", "failed"].includes(transfer.state));
    const stopActiveTransfers = activeTransfers.length > 0;
    if (stopActiveTransfers && !window.confirm(`还有 ${activeTransfers.length} 个传输任务正在运行。断开连接会取消这些任务，是否继续？`)) return;

    await closeAllTerminals();
    const snapshot = await invoke("connection_disconnect", { connectionId: entry.connectionId, stopActiveTransfers });
    entry.snapshot = snapshot;
    entry.watching = false;
    state.connections.delete(entry.serverId);
    state.activeWorkspace = null;
    $("#nav-workspace").disabled = true;
    $("#nav-live-dot").hidden = true;
    showPage("servers");
    renderAll();
    renderStats();
    toast("已安全断开 SSH 连接。");
  } catch (error) {
    toast(errorMessage(error), "error", 7000);
  } finally {
    state.disconnecting = false;
    $("#disconnect-server").disabled = false;
    renderFiles();
  }
}

function openGroupDialog() {
  $("#group-form").reset();
  $("#group-form-error").hidden = true;
  openDialog($("#group-dialog"));
  window.setTimeout(() => $("#group-name").focus(), 20);
}

function setTerminalFocus(enabled) {
  const shell = $(".app-shell");
  shell.classList.toggle("is-focused", enabled);
  const button = $("#focus-terminal");
  button.setAttribute("aria-pressed", String(enabled));
  button.innerHTML = `${iconMarkup(enabled ? "minimize" : "maximize-2")}<span>${enabled ? "退出专注" : "专注"}</span>`;
  requestAnimationFrame(() => fitTerminal(state.terminals.get(state.activeTerminalId)));
}

async function createGroup(event) {
  event.preventDefault();
  const name = $("#group-name").value.trim();
  if (!name) {
    $("#group-form-error").textContent = "请输入分组名称。";
    $("#group-form-error").hidden = false;
    return;
  }
  try {
    await invoke("group_create", { name, sortOrder: state.groups.length });
    await loadData();
    renderGroups();
    renderServers();
    closeDialog($("#group-dialog"));
    toast("服务器分组已创建。");
  } catch (error) {
    $("#group-form-error").textContent = errorMessage(error);
    $("#group-form-error").hidden = false;
  }
}

function openDeleteDialog(profile) {
  state.deleteTarget = profile;
  setText($("#confirm-title"), `删除“${profile.name}”？`);
  setText($("#confirm-description"), "服务器配置和系统安全存储中的凭据都会被删除。此操作无法撤销。");
  openDialog($("#confirm-dialog"));
}

async function deleteServer() {
  const profile = state.deleteTarget;
  if (!profile) return;
  $("#confirm-delete").disabled = true;
  try {
    await invoke("server_delete", { serverId: profile.id, expectedRevision: profile.revision, removeCredentials: true });
    state.deleteTarget = null;
    state.restoreEditorAfterDeleteConfirmation = false;
    closeDialog($("#confirm-dialog"));
    await loadData();
    toast("服务器与已保存凭据已删除。");
  } catch (error) {
    toast(errorMessage(error), "error", 7000);
  } finally {
    $("#confirm-delete").disabled = false;
  }
}

function platformLabel(value) {
  return ({ macos: "macOS", windows: "Windows" })[value] ?? value ?? "—";
}

async function openAbout() {
  try {
    const info = await core.invoke("app_get_info");
    state.appInfo = info;
    setText($("#about-version"), info.version);
    setText($("#about-platform"), platformLabel(info.platform));
    setText($("#about-architecture"), info.architecture);
  } catch (error) {
    toast(errorMessage(error), "error");
  }
  openDialog($("#about-dialog"));
}

function bindEvents() {
  const paletteDialog = $("#command-palette");
  const paletteInput = $("#command-palette-input");
  const paletteResults = $("#command-palette-results");
  paletteInput.addEventListener("input", renderCommandPalette);
  paletteInput.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      commandPaletteState.activeIndex = movePaletteSelection(
        commandPaletteState.visible,
        commandPaletteState.activeIndex,
        event.key === "ArrowDown" ? 1 : -1,
      );
      syncPaletteSelection();
    } else if (event.key === "Enter") {
      event.preventDefault();
      activatePaletteCommand(commandPaletteState.visible[commandPaletteState.activeIndex]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeCommandPalette();
    }
  });
  paletteResults.addEventListener("click", (event) => {
    const option = event.target.closest("[data-palette-index]");
    if (!option) return;
    activatePaletteCommand(commandPaletteState.visible[Number(option.dataset.paletteIndex)]);
  });
  paletteResults.addEventListener("pointermove", (event) => {
    const option = event.target.closest("[data-palette-index]");
    const index = Number(option?.dataset.paletteIndex);
    if (!option || !Number.isInteger(index) || commandPaletteState.visible[index]?.disabled) return;
    commandPaletteState.activeIndex = index;
    syncPaletteSelection();
  });
  paletteDialog.addEventListener("cancel", (event) => {
    event.preventDefault();
    closeCommandPalette();
  });
  paletteDialog.addEventListener("click", (event) => {
    if (event.target === paletteDialog) closeCommandPalette();
  });
  paletteDialog.addEventListener("close", () => {
    paletteInput.setAttribute("aria-expanded", "false");
    paletteInput.removeAttribute("aria-activedescendant");
    if (commandPaletteState.restoreFocus && commandPaletteState.returnFocus instanceof HTMLElement && commandPaletteState.returnFocus.isConnected) {
      commandPaletteState.returnFocus.focus();
    }
    commandPaletteState.returnFocus = null;
    commandPaletteState.restoreFocus = true;
  });
  $("#workspace-terminal-tab").addEventListener("click", () => setWorkspaceMode("terminal"));
  $("#workspace-files-tab").addEventListener("click", () => setWorkspaceMode("files"));
  $("#workspace-monitor-tab").addEventListener("click", () => setWorkspaceMode("monitor"));
  $("#quick-open-monitor").addEventListener("click", () => setWorkspaceMode("monitor"));
  $("#focus-terminal").addEventListener("click", () => setTerminalFocus(!$(".app-shell").classList.contains("is-focused")));
  $("#files-open-full").addEventListener("click", () => setWorkspaceMode("files"));
  $("#files-back-terminal").addEventListener("click", () => setWorkspaceMode("terminal"));
  $("#monitor-back-terminal").addEventListener("click", () => setWorkspaceMode("terminal"));
  $("#monitor-refresh").addEventListener("click", () => void refreshMonitor());
  $("#files-parent").addEventListener("click", () => navigateFiles(parentRemotePath(state.files.path)));
  $("#files-path-breadcrumb").addEventListener("click", (event) => {
    const segment = event.target.closest("[data-remote-path]");
    if (segment && !segment.disabled) navigateFiles(segment.dataset.remotePath);
  });
  $("#files-edit-path").addEventListener("click", () => {
    const pathBar = $("#files-path-bar");
    const input = $("#files-path-input");
    const editing = !pathBar.classList.contains("is-editing");
    pathBar.classList.toggle("is-editing", editing);
    $(".files-path-input-wrap").hidden = !editing;
    $("#files-go").hidden = !editing;
    $("#files-edit-path").setAttribute("aria-expanded", String(editing));
    if (editing) { input.focus(); input.select(); }
  });
  $("#files-go").addEventListener("click", () => navigateFiles($("#files-path-input").value.trim() || "."));
  $("#files-path-input").addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      navigateFiles(event.currentTarget.value.trim() || ".");
    }
  });
  $("#files-refresh").addEventListener("click", () => navigateFiles(state.files.path || "."));
  $("#files-clear-completed").addEventListener("click", clearCompletedTransfers);
  $("#files-load-more").addEventListener("click", () => void loadFiles({ append: true }));
  $("#files-upload").addEventListener("click", () => void selectUpload());
  $("#files-download").addEventListener("click", () => void selectDownload());
  $("#files-copy-path").addEventListener("click", () => void copySelectedPath());
  $("#files-new-folder").addEventListener("click", () => openFileNameDialog("mkdir"));
  $("#files-rename").addEventListener("click", () => openFileNameDialog("rename"));
  $("#files-delete").addEventListener("click", openFileDeleteDialog);
  $("#file-name-form").addEventListener("submit", (event) => void submitFileName(event));
  $("#file-delete-cancel").addEventListener("click", () => closeDialog($("#file-delete-dialog")));
  $("#file-delete-confirm").addEventListener("click", () => void deleteSelectedFile());
  $("#add-server").addEventListener("click", () => openServerDialog());
  $("#sidebar-add-server").addEventListener("click", () => openServerDialog());
  $("#welcome-add-server").addEventListener("click", () => openServerDialog());
  $("#add-group").addEventListener("click", openGroupDialog);
  $("#server-search").addEventListener("input", (event) => {
    updateServerSearch(event.target.value, "sidebar");
  });
  $("#global-search").addEventListener("input", (event) => updateServerSearch(event.target.value, "global"));
  $("#profile-auth").addEventListener("change", updateAuthFields);
  $("#proxy-type").addEventListener("change", updateProxyFields);
  for (const choice of $$('[data-auth-type-choice]')) {
    choice.addEventListener("click", () => {
      $("#profile-auth").value = choice.dataset.authTypeChoice;
      $("#profile-auth").dispatchEvent(new Event("change", { bubbles: true }));
    });
  }
  $("#toggle-secret-visibility").addEventListener("click", (event) => {
    const button = event.currentTarget;
    const secret = $("#profile-secret");
    const reveal = secret.type === "password";
    secret.type = reveal ? "text" : "password";
    button.setAttribute("aria-label", reveal ? "隐藏密码" : "显示密码");
    button.setAttribute("aria-pressed", String(reveal));
    button.innerHTML = iconMarkup(reveal ? "eye-off" : "eye");
  });
  $("#select-key").addEventListener("click", () => void selectPrivateKey());
  $("#clear-key").addEventListener("click", () => {
    state.selectedKey = null;
    $("#selected-key").textContent = state.editingProfile?.hasPrivateKey ? "当前私钥将在未重新选择时保留" : "尚未选择文件";
    $("#clear-key").hidden = true;
  });
  $("#server-form").addEventListener("submit", (event) => {
    event.preventDefault();
    void saveProfile(false);
  });
  $("#test-connection").addEventListener("click", () => void testProfileConnection());
  $("#save-and-connect").addEventListener("click", () => void saveProfile(true));
  $("#edit-delete-button").addEventListener("click", () => {
    const profile = state.editingProfile;
    if (!profile) return;
    state.restoreEditorAfterDeleteConfirmation = true;
    closeDialog($("#server-dialog"));
    openDeleteDialog(profile);
  });
  $("#connection-error-close").addEventListener("click", () => {
    state.connectionFailureProfile = null;
    closeDialog($("#connection-error-dialog"));
  });
  $("#connection-error-retry").addEventListener("click", () => {
    const profile = state.connectionFailureProfile;
    closeDialog($("#connection-error-dialog"));
    if (profile) void startSavedConnection(profile, "workspace");
  });
  $("#connection-error-edit").addEventListener("click", () => {
    const profile = state.connectionFailureProfile;
    closeDialog($("#connection-error-dialog"));
    if (profile) openServerDialog(profile);
  });
  $("#group-form").addEventListener("submit", (event) => void createGroup(event));
  $("#confirm-delete").addEventListener("click", () => void deleteServer());
  $("#cancel-delete").addEventListener("click", () => {
    closeDialog($("#confirm-dialog"));
    if (state.restoreEditorAfterDeleteConfirmation && state.deleteTarget) openServerDialog(state.deleteTarget);
    state.restoreEditorAfterDeleteConfirmation = false;
  });
  $("#trust-host-once").addEventListener("click", () => void respondHostKey("trustOnce"));
  $("#trust-host-save").addEventListener("click", () => void respondHostKey("trustAndSave"));
  $("#reject-host-key").addEventListener("click", () => void respondHostKey("reject"));
  $("#update-host-key").addEventListener("click", () => void respondHostKey("trustAndSave"));
  $("#submit-auth").addEventListener("click", () => void submitAuthentication());
  $("#challenge-auth-form").addEventListener("submit", (event) => {
    event.preventDefault();
    void submitAuthentication();
  });
  $("#cancel-auth").addEventListener("click", async () => {
    const entry = state.activeChallenge?.entry;
    if (entry) await cancelConnection(entry);
    state.activeChallenge = null;
    closeDialog($("#challenge-dialog"));
  });
  $("#nav-servers").addEventListener("click", () => showPage("servers"));
  $("#nav-workspace").addEventListener("click", () => {
    if (state.activeWorkspace) enterWorkspace(state.activeWorkspace);
  });
  $("#nav-settings").addEventListener("click", () => showPage("settings"));
  $("#back-to-servers").addEventListener("click", () => showPage("servers"));
  $("#disconnect-server").addEventListener("click", () => void disconnectWorkspace());
  for (const button of $$("[data-settings-target]")) {
    button.addEventListener("click", () => setSettingsCategory(button.dataset.settingsTarget));
  }
  for (const button of $$("[data-theme-choice]")) {
    button.addEventListener("click", () => {
      const theme = button.dataset.themeChoice;
      document.documentElement.dataset.theme = theme;
      for (const choice of $$("[data-theme-choice]")) {
        choice.setAttribute("aria-pressed", String(choice === button));
      }
      void saveSettings({ theme });
    });
  }
  $("#setting-language").addEventListener("click", (event) => {
    const choice = event.target.closest("[data-language-choice]");
    if (!choice) return;
    const language = choice.dataset.languageChoice;
    for (const choice of $$("[data-language-choice]")) {
      choice.setAttribute("aria-pressed", String(choice.dataset.languageChoice === language));
    }
    if (language === state.settings.value.language) return;
    setLocale(language);
    void saveSettings({ language });
  });
  $("#setting-confirm-disconnect").addEventListener("change", (event) => {
    void saveSettings({ confirmBeforeDisconnect: event.currentTarget.checked });
  });
  $("#setting-terminal-font").addEventListener("change", (event) => {
    const terminalFontFamily = event.currentTarget.value.trim();
    if (!terminalFontFamily) {
      renderSettings();
      toast("终端字体不能为空。", "error");
      return;
    }
    void saveSettings({ terminalFontFamily });
  });
  $("#setting-terminal-size").addEventListener("input", (event) => {
    updateTerminalFontSizeControls(event.currentTarget.value);
  });
  $("#setting-terminal-size").addEventListener("change", (event) => {
    void saveSettings({ terminalFontSize: Number(event.currentTarget.value) });
  });
  $("#setting-terminal-size-presets").addEventListener("click", (event) => {
    const choice = event.target.closest("[data-terminal-font-size]");
    if (!choice) return;
    const terminalFontSize = Number(choice.dataset.terminalFontSize);
    updateTerminalFontSizeControls(terminalFontSize);
    void saveSettings({ terminalFontSize });
  });
  $("#setting-terminal-cursor").addEventListener("click", (event) => {
    const choice = event.target.closest("[data-terminal-cursor]");
    if (!choice) return;
    const terminalCursorStyle = choice.dataset.terminalCursor;
    for (const option of $$("#setting-terminal-cursor [data-terminal-cursor]")) {
      option.setAttribute("aria-pressed", String(option === choice));
    }
    void saveSettings({ terminalCursorStyle });
  });
  $("#setting-terminal-scrollback").addEventListener("change", (event) => {
    const terminalScrollbackLines = Number(event.currentTarget.value);
    if (!Number.isInteger(terminalScrollbackLines) || terminalScrollbackLines < 1000 || terminalScrollbackLines > 100_000) {
      renderSettings();
      toast("滚动缓冲区必须在 1,000 到 100,000 行之间。", "error");
      return;
    }
    void saveSettings({ terminalScrollbackLines });
  });
  $("#setting-terminal-copy-on-select").addEventListener("change", (event) => {
    setTerminalCopyOnSelectEnabled(event.currentTarget.checked, localStorage);
  });
  $("#new-terminal").addEventListener("click", () => void openTerminal());
  $("#clear-terminal").addEventListener("click", () => {
    const entry = state.terminals.get(state.activeTerminalId);
    if (entry) entry.instance.clear();
  });
  const resizeTerminal = () => fitTerminal(state.terminals.get(state.activeTerminalId));
  if (typeof ResizeObserver !== "undefined") {
    const observer = new ResizeObserver(resizeTerminal);
    observer.observe($("#terminal-screen"));
  } else {
    window.addEventListener("resize", resizeTerminal);
  }
  $("#about-button").addEventListener("click", () => void openAbout());
  for (const button of $$('[data-close]')) {
    button.addEventListener("click", () => closeDialog($(`#${CSS.escape(button.dataset.close)}`)));
  }
  for (const dialog of $$('dialog:not(#challenge-dialog)')) {
    dialog.addEventListener("click", (event) => {
      if (event.target === dialog) closeDialog(dialog);
    });
  }
  $("#challenge-dialog").addEventListener("cancel", (event) => {
    event.preventDefault();
    const active = state.activeChallenge;
    if (!active) return;
    if (active.type === "hostKey") void respondHostKey("reject");
    else void $("#cancel-auth").click();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && $(".app-shell").classList.contains("is-focused")) {
      setTerminalFocus(false);
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      if (paletteDialog.open) closeCommandPalette();
      else openCommandPalette();
    }
  });
}

async function boot() {
  hydrateIcons();
  bindEvents();
  renderSettings();
  if (!core?.invoke || !core?.Channel) {
    setBackendStatus("error", "请在 MauLink 桌面应用中运行");
    $("#server-list").append(makeEmptyState("桌面服务未连接", "服务器配置存放在本机安全数据库中。请从 MauLink 应用启动此页面。", null, null));
    return;
  }
  try {
    const info = await core.invoke("app_get_info");
    state.appInfo = info;
    setText($("#app-version"), `v${info.version}`);
    setText($("#footer-version"), info.version);
    setText($("#search-shortcut"), info.platform === "windows" ? "Ctrl K" : "⌘ K");
    setText($("#settings-version"), info.version);
    try {
      state.settings = await invoke("settings_get", {});
      renderSettings();
      applyTerminalSettings(state.settings.value);
    } catch (error) {
      setText($("#settings-save-state"), "无法读取设置");
      toast(`无法读取设置：${errorMessage(error)}`, "error", 7000);
    }
    await loadData();
  } catch {
    // loadData reports the recoverable backend error in the page and toast.
  }
}

void boot();
