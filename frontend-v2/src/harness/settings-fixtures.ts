import type { AppError } from '../../../contracts/v1/AppError';
import type { SettingsRecord } from '../../../contracts/v1/SettingsRecord';
import { defaultSettings } from '../terminal/preferences';
import { createMockIpc } from '../ipc/mock';
export function createSettingsMock(options: { readFailure?: () => boolean; conflict?: () => boolean; changed?: (value: SettingsRecord) => void } = {}) {
  let record: SettingsRecord = { value: { ...defaultSettings }, revision: 1, updatedAtMs: 1 };
  const failure = (code: 'REVISION_CONFLICT' | 'INTERNAL', messageKey: string): AppError => ({ code, messageKey, params: {}, retryable: false, action: 'reload', stage: 'storage', requestId: null, details: null });
  const transport = createMockIpc({
    settings_get: () => { if (options.readFailure?.()) throw failure('INTERNAL', 'errors.settingsDataInvalid'); return structuredClone(record); },
    settings_update: payload => { if (options.conflict?.() || payload.expectedRevision !== record.revision) throw failure('REVISION_CONFLICT', 'errors.revisionConflict'); record = { value: structuredClone(payload.value), revision: record.revision + 1, updatedAtMs: Date.now() }; options.changed?.(record); return structuredClone(record); },
  });
  return { transport, current: () => structuredClone(record) };
}
