export interface InlineFilesPreference {
  expanded: boolean;
  ratio: number;
}

interface WorkspaceLayoutV1 {
  version: 1;
  servers: Record<string, InlineFilesPreference>;
}

export interface InlineFilesLayout {
  available: boolean;
  filesHeight: number;
  minFilesHeight: number;
  maxFilesHeight: number;
}

export const WORKSPACE_LAYOUT_KEY = "maulink.workspace.layout.v1";
export const DEFAULT_INLINE_FILES_PREFERENCE: InlineFilesPreference = { expanded: false, ratio: 0.3 };

const MIN_RATIO = 0.2;
const MAX_RATIO = 0.42;
const TERMINAL_MIN_HEIGHT = 240;
const SPLITTER_HEIGHT = 6;
const MAX_SAVED_SERVERS = 100;

export function normalizeInlineFilesRatio(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_INLINE_FILES_PREFERENCE.ratio;
  return Math.min(MAX_RATIO, Math.max(MIN_RATIO, value));
}

function validPreference(value: unknown): value is InlineFilesPreference {
  if (!value || typeof value !== "object") return false;
  const preference = value as Partial<InlineFilesPreference>;
  return typeof preference.expanded === "boolean" && typeof preference.ratio === "number" && Number.isFinite(preference.ratio);
}

function readRecord(): WorkspaceLayoutV1 {
  try {
    const raw = localStorage.getItem(WORKSPACE_LAYOUT_KEY);
    if (!raw) return { version: 1, servers: {} };
    const value = JSON.parse(raw) as Partial<WorkspaceLayoutV1>;
    if (value.version !== 1 || !value.servers || typeof value.servers !== "object" || Array.isArray(value.servers)) return { version: 1, servers: {} };
    const servers = Object.fromEntries(Object.entries(value.servers).filter((entry): entry is [string, InlineFilesPreference] => validPreference(entry[1]))
      .map(([serverId, preference]) => [serverId, { expanded: preference.expanded, ratio: normalizeInlineFilesRatio(preference.ratio) }]));
    return { version: 1, servers };
  } catch {
    return { version: 1, servers: {} };
  }
}

export function readInlineFilesPreference(serverId: string): InlineFilesPreference {
  if (!serverId) return { ...DEFAULT_INLINE_FILES_PREFERENCE };
  const preference = readRecord().servers[serverId];
  return preference ? { ...preference } : { ...DEFAULT_INLINE_FILES_PREFERENCE };
}

export function saveInlineFilesPreference(serverId: string, preference: InlineFilesPreference): void {
  if (!serverId || !validPreference(preference)) return;
  try {
    const record = readRecord();
    delete record.servers[serverId];
    record.servers[serverId] = { expanded: preference.expanded, ratio: normalizeInlineFilesRatio(preference.ratio) };
    const retained = Object.entries(record.servers).slice(-MAX_SAVED_SERVERS);
    localStorage.setItem(WORKSPACE_LAYOUT_KEY, JSON.stringify({ version: 1, servers: Object.fromEntries(retained) } satisfies WorkspaceLayoutV1));
  } catch {
    // Layout preferences are optional and must never prevent terminal or SFTP use.
  }
}

export function resolveInlineFilesLayout(mainHeight: number, ratio: number, compact: boolean): InlineFilesLayout {
  const minFilesHeight = compact ? 148 : 160;
  const minimumMainHeight = minFilesHeight + TERMINAL_MIN_HEIGHT + SPLITTER_HEIGHT;
  if (!Number.isFinite(mainHeight) || mainHeight < minimumMainHeight) {
    return { available: false, filesHeight: 0, minFilesHeight, maxFilesHeight: 0 };
  }
  const maxFilesHeight = Math.floor(Math.min(mainHeight * MAX_RATIO, mainHeight - TERMINAL_MIN_HEIGHT - SPLITTER_HEIGHT));
  const filesHeight = Math.round(Math.min(maxFilesHeight, Math.max(minFilesHeight, mainHeight * normalizeInlineFilesRatio(ratio))));
  return { available: true, filesHeight, minFilesHeight, maxFilesHeight };
}
