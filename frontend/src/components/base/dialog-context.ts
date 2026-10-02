import type { InjectionKey } from 'vue';
// A non-interactive tooltip must not consume its parent modal's Escape action.
export const dialogEscapeKey: InjectionKey<() => void> = Symbol('dialogEscape');
