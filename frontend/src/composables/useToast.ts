import { ref } from 'vue';
export type ToastKind = 'success' | 'info' | 'warning' | 'error';
export interface ToastMessage {
  id: string; kind: ToastKind; title?: string; description: string; duration: number; open: boolean;
  action?: { label: string; run: () => void };
}
type ToastOptions = Partial<Pick<ToastMessage, 'title' | 'duration' | 'action'>>;
const durations: Record<ToastKind, number> = { success:3000, info:3500, warning:4500, error:5500 };
const maxVisible = 3;
const dedupeWindowMs = 2000;
export function useToast() {
  const messages = ref<ToastMessage[]>([]);
  const pending: ToastMessage[] = [];
  const recent = new Map<string, { id: string; at: number }>();
  let sequence = 0;
  function add(kind: ToastKind, description: string, options: ToastOptions = {}) {
    const now = Date.now();
    for (const [key, entry] of recent) if (now - entry.at >= dedupeWindowMs) recent.delete(key);
    if (!options.action) {
      const dedupeKey = JSON.stringify([kind, options.title ?? '', description]);
      const previous = recent.get(dedupeKey);
      if (previous && now - previous.at < dedupeWindowMs) {
        previous.at = now;
        return previous.id;
      }
    }
    const id = String(++sequence);
    const message = { ...options, id, kind, description, duration:options.duration ?? durations[kind], open:true };
    if (messages.value.length < maxVisible) messages.value.push(message);
    else pending.push(message);
    if (!options.action) recent.set(JSON.stringify([kind, options.title ?? '', description]), { id, at: now });
    return id;
  }
  function dismiss(id: string) {
    const message = messages.value.find(item => item.id === id);
    if (message) { message.open = false; return; }
    const pendingIndex = pending.findIndex(item => item.id === id);
    if (pendingIndex >= 0) pending.splice(pendingIndex, 1);
  }
  function remove(id: string) {
    const visibleIndex = messages.value.findIndex(item => item.id === id);
    if (visibleIndex < 0) {
      const pendingIndex = pending.findIndex(item => item.id === id);
      if (pendingIndex >= 0) pending.splice(pendingIndex, 1);
      return;
    }
    messages.value.splice(visibleIndex, 1);
    const next = pending.shift();
    if (next) messages.value.push(next);
  }
  return { messages, dismiss, remove,
    success:(description: string, options?: ToastOptions) => add('success',description,options),
    info:(description: string, options?: ToastOptions) => add('info',description,options),
    warning:(description: string, options?: ToastOptions) => add('warning',description,options),
    error:(description: string, options?: ToastOptions) => add('error',description,options),
  };
}
export type ToastQueue = ReturnType<typeof useToast>;
