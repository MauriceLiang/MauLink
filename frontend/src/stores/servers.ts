import { computed, ref } from "vue";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerCreatePayload } from "../../../contracts/v1/ServerCreatePayload";
import type { ServerUpdatePayload } from "../../../contracts/v1/ServerUpdatePayload";
import type { GroupUpdatePayload } from "../../../contracts/v1/GroupUpdatePayload";
import type { RevisionPayload } from "../../../contracts/v1/RevisionPayload";
import type { ServerDeletePayload } from "../../../contracts/v1/ServerDeletePayload";
import type { GroupCreate } from "../../../contracts/v1/GroupCreate";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import type { createServerApi } from "../ipc/server";

export function createServerStore(api: ReturnType<typeof createServerApi>) {
  const servers = ref<ServerProfile[]>([]);
  const groups = ref<Group[]>([]);
  const query = ref("");
  const pending = ref(true);
  const error = ref<ReturnType<typeof presentError> | null>(null);
  const filtered = computed(() => {
    const needle = query.value.trim().toLocaleLowerCase();
    return servers.value.filter(server => `${server.name} ${server.host} ${server.username}`.toLocaleLowerCase().includes(needle));
  });

  async function load() {
    pending.value = true;
    error.value = null;
    try {
      const nextGroups = await api.listGroups();
      const items: ServerProfile[] = [];
      let cursor: string | null = null;
      do {
        const page = await api.list({ query: null, groupId: null, limit: 200, cursor });
        items.push(...page.items);
        cursor = page.nextCursor;
      } while (cursor);
      groups.value = nextGroups;
      servers.value = items;
      return true;
    } catch (failure) {
      error.value = presentError(mapError(failure));
      return false;
    } finally {
      pending.value = false;
    }
  }

  function upsert(server: ServerProfile) {
    const index = servers.value.findIndex(item => item.id === server.id);
    if (index < 0) servers.value.push(server);
    else servers.value[index] = server;
  }
  function upsertGroup(group: Group) {
    groups.value = [...groups.value.filter(item => item.id !== group.id), group]
      .sort((left, right) => left.sortOrder - right.sortOrder || left.name.localeCompare(right.name));
  }

  return {
    servers, groups, query, filtered, pending, error, load,
    getServer: (id: string) => api.get({ id }),
    selectPrivateKey: () => api.selectLocalFile({ purpose: "privateKey" }),
    async create(payload: ServerCreatePayload) {
      const result = await api.create(payload);
      upsert(result.server);
      return result;
    },
    async update(payload: ServerUpdatePayload) {
      const result = await api.update(payload);
      upsert(result.server);
      return result;
    },
    async remove(payload: ServerDeletePayload) {
      const result = await api.remove(payload);
      servers.value = servers.value.filter(server => server.id !== payload.serverId);
      return result;
    },
    async createGroup(payload: GroupCreate) {
      upsertGroup(await api.createGroup(payload));
    },
    async updateGroup(payload: GroupUpdatePayload) {
      upsertGroup(await api.updateGroup(payload));
    },
    async removeGroup(payload: RevisionPayload) {
      await api.removeGroup(payload);
      // The existing SQLite foreign key moves members to the ungrouped section.
      groups.value = groups.value.filter(group => group.id !== payload.id);
      servers.value = servers.value.map(server => server.groupId === payload.id ? { ...server, groupId: null } : server);
    },
  };
}

export type ServerStore = ReturnType<typeof createServerStore>;
