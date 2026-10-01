import { ref } from "vue";
import type { Language } from "../../../contracts/v1/Language";
export const locale = ref<Language>('zh-CN');
export type Messages = Record<string, readonly [string, string]>;
export function messages<T extends Messages>(catalog: T) {
  return (key: keyof T, params: Record<string, string | number> = {}) => catalog[key]![locale.value === 'en' ? 1 : 0]!.replace(/\{(\w+)\}/g, (_, name: string) => String(params[name] ?? '—'));
}
