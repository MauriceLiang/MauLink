import { ref } from 'vue';
export type ToastKind = 'success' | 'info' | 'warning' | 'error';
export interface ToastMessage {
  id: string; kind: ToastKind; title?: string; description: string; duration: number; open: boolean;
  action?: { label: string; run: () => void };
}
type ToastOptions = Partial<Pick<ToastMessage, 'title' | 'duration' | 'action'>>;
const durations: Record<ToastKind, number> = { success:3000, info:3500, warning:4500, error:5500 };
export function useToast() {
  const messages = ref<ToastMessage[]>([]);
  let sequence = 0;
  function add(kind: ToastKind, description: string, options: ToastOptions = {}) {
    const id = String(++sequence);
    messages.value.push({ ...options, id, kind, description, duration:options.duration ?? durations[kind], open:true });
    return id;
  }
  function dismiss(id: string) { const message = messages.value.find(item => item.id === id); if (message) message.open = false; }
  function remove(id: string) { messages.value = messages.value.filter(item => item.id !== id); }
  return { messages, dismiss, remove,
    success:(description: string, options?: ToastOptions) => add('success',description,options),
    info:(description: string, options?: ToastOptions) => add('info',description,options),
    warning:(description: string, options?: ToastOptions) => add('warning',description,options),
    error:(description: string, options?: ToastOptions) => add('error',description,options),
  };
}
export type ToastQueue = ReturnType<typeof useToast>;
