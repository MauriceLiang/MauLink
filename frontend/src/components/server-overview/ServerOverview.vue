<script setup lang="ts">
import { computed } from "vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";
import type { ConnectionStore } from "../../stores/connections";
import type { MenuItem } from "../base/menu";
import BaseButton from "../base/BaseButton.vue";
import ConnectionPanel from "../connection/ConnectionPanel.vue";
import ConnectionInfoCard from "./ConnectionInfoCard.vue";
import ConnectionRouteCard from "./ConnectionRouteCard.vue";
import HostIdentityCard from "./HostIdentityCard.vue";
import ServerOverviewHeader from "./ServerOverviewHeader.vue";
import { buildSshCommand, formatServerEndpoint } from "./server-details";
import type { createHostKeysApi } from "../../ipc/host-keys";

const props = defineProps<{ server: ServerProfile; store: ConnectionStore; hostKeyApi: ReturnType<typeof createHostKeysApi>; readOnly: boolean }>();
const emit = defineEmits<{ back: []; edit: [id: string]; remove: [server: ServerProfile]; copy: [kind: "address" | "ssh", value: string] }>();
const t = messages(serverOverviewMessages);
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const sshCommand = computed(() => buildSshCommand(props.server));
const menuItems = computed<MenuItem[]>(() => [
  { id: "edit", label: t("editServer"), disabled: props.readOnly },
  { id: "copy-address", label: t("copyAddress") },
  { id: "copy-ssh", label: t("copySshCommand"), disabled: !sshCommand.value, title: sshCommand.value ? undefined : t("proxyCommandUnavailable") },
  { id: "delete", label: t("deleteServer"), disabled: props.readOnly, danger: true, separatorBefore: true },
]);

function menuAction(id: string) {
  if (id === "edit") emit("edit", props.server.id);
  else if (id === "delete") emit("remove", props.server);
  else if (id === "copy-address") emit("copy", "address", formatServerEndpoint(props.server.username, props.server.host, props.server.port));
  else if (id === "copy-ssh" && sshCommand.value) emit("copy", "ssh", sshCommand.value);
}
</script>

<template>
  <section class="server-overview">
    <ServerOverviewHeader :server="server" :state="snapshot?.state" :menu-items="menuItems" @back="emit('back')" @menu-action="menuAction">
      <template #connection-actions>
        <ConnectionPanel :server="server" :store="store" :read-only="readOnly" :connect-label="t('connect')">
          <BaseButton :disabled="true" :title="t('connectionCheck')">{{ t('connectionCheck') }}</BaseButton>
        </ConnectionPanel>
      </template>
    </ServerOverviewHeader>
    <div class="server-overview-grid">
      <ConnectionInfoCard :server="server" />
      <HostIdentityCard :server="server" :api="hostKeyApi" />
      <ConnectionRouteCard :server="server" />
    </div>
  </section>
</template>
