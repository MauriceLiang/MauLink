<script setup lang="ts">
import type { ConnectionState } from "../../../../contracts/v1/ConnectionState";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { messages } from "../../i18n/locale";
import { connectionStateText } from "../../i18n/connections";
import { serverOverviewMessages } from "../../i18n/server-overview";
import BaseButton from "../base/BaseButton.vue";
import BaseDropdownMenu from "../base/BaseDropdownMenu.vue";
import BaseStatusBadge from "../base/BaseStatusBadge.vue";
import type { MenuItem } from "../base/menu";
import { formatServerEndpoint } from "./server-details";

const props = defineProps<{ server: ServerProfile; state?: ConnectionState; menuItems: MenuItem[] }>();
const emit = defineEmits<{ back: []; menuAction: [id: string] }>();
const t = messages(serverOverviewMessages);
function statusText() { return props.state ? connectionStateText(props.state) : t("notConnected"); }
function statusTone(): "neutral" | "success" | "error" {
  if (props.state === "ready") return "success";
  if (props.state === "failed") return "error";
  return "neutral";
}
</script>

<template>
  <header class="server-overview-header">
    <div class="server-overview-toolbar">
      <BaseButton class="server-overview-back" @click="emit('back')"><span aria-hidden="true">←</span>{{ t('backToServers') }}</BaseButton>
      <div class="server-overview-actions">
        <slot name="connection-actions" />
        <BaseDropdownMenu :label="`${t('moreActions')} · ${server.name}`" :items="menuItems" @action="emit('menuAction', $event)" />
      </div>
    </div>
    <div class="server-overview-identity">
      <div class="server-overview-title-line">
        <h1>{{ server.name }}</h1>
        <BaseStatusBadge :tone="statusTone()">{{ statusText() }}</BaseStatusBadge>
      </div>
      <p class="server-overview-endpoint">{{ formatServerEndpoint(server.username, server.host, server.port) }}</p>
    </div>
  </header>
</template>
