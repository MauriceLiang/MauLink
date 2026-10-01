import { ref, shallowRef } from "vue";
import type { AppSettings } from "../../../contracts/v1/AppSettings";
import type { SettingsRecord } from "../../../contracts/v1/SettingsRecord";
import type { createSettingsApi } from "../ipc/settings";
import { mapError } from "../errors/mapper";
import { locale } from '../i18n/locale';
import { presentError } from "../errors/presenter";
export const defaultSettings: AppSettings = { theme: 'system', language: 'zh-CN', terminalFontFamily: 'monospace', terminalFontSize: 14, terminalCursorStyle: 'block', terminalScrollbackLines: 10000, downloadDirectoryToken: null, confirmBeforeDisconnect: true };
const copyKey = 'maulink.terminal.copyOnSelect';
export function createTerminalPreferences(api: ReturnType<typeof createSettingsApi>, apply: (settings: AppSettings) => void) {
  const record = shallowRef<SettingsRecord | null>(null);
  const busy = ref(false);
  const error = ref('');
  const copyOnSelect = ref(false);
  try { copyOnSelect.value = localStorage.getItem(copyKey) === 'true'; } catch { /* No browser storage means the default remains disabled. */ }
  let loading: Promise<void> | null = null;
  function load() {
    if (loading) return loading;
    busy.value = true; error.value = '';
    loading = api.get().then(value => { record.value = value; apply(value.value); }).catch(reason => { error.value = presentError(mapError(reason)).message; }).finally(() => { busy.value = false; loading = null; });
    return loading;
  }
  async function save(patch: Partial<AppSettings>, copy?: boolean) {
    if (busy.value || !record.value) return false;
    busy.value = true; error.value = '';
    try {
      const next = await api.update({ expectedRevision: record.value.revision, value: { ...record.value.value, ...patch } });
      record.value = next; apply(next.value);
      try { if (copy !== undefined) { localStorage.setItem(copyKey, String(copy)); copyOnSelect.value = copy; } }
      catch { error.value = locale.value === 'en' ? 'Settings saved, but the copy-on-select preference could not be saved.' : '终端设置已保存，但选中即复制偏好未能保存。'; }
      return true;
    } catch (reason) { error.value = presentError(mapError(reason)).message; return false; }
    finally { busy.value = false; }
  }
  return { record, busy, error, copyOnSelect, load, save };
}
export type TerminalPreferences = ReturnType<typeof createTerminalPreferences>;
