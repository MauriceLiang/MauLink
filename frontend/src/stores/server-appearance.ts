import { shallowRef } from "vue";
import type { ServerAppearance } from "../../../contracts/v1/ServerAppearance";
import type { ServerAppearanceUpdate } from "../../../contracts/v1/ServerAppearanceUpdate";
import type { createServerAppearanceApi } from "../ipc/server-appearance";

export function createServerAppearanceStore(api: ReturnType<typeof createServerAppearanceApi>) {
  const appearances = shallowRef<Record<string, ServerAppearance>>({});

  function replace(items: ServerAppearance[]) {
    appearances.value = Object.fromEntries(items.map(item => [item.serverId, item]));
  }

  return {
    appearances,
    async load() {
      replace(await api.list());
    },
    async get(serverId: string) {
      const appearance = await api.get({ serverId });
      appearances.value = { ...appearances.value, [serverId]: appearance };
      return appearance;
    },
    async update(payload: ServerAppearanceUpdate) {
      const appearance = await api.update(payload);
      appearances.value = { ...appearances.value, [appearance.serverId]: appearance };
      return appearance;
    },
    remove(serverId: string) {
      const next = { ...appearances.value };
      delete next[serverId];
      appearances.value = next;
    },
  };
}

export type ServerAppearanceStore = ReturnType<typeof createServerAppearanceStore>;
