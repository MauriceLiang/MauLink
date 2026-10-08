<script setup lang="ts">
import type { ConnectionState } from "../../../../contracts/v1/ConnectionState";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { computed } from "vue";
import { messages } from "../../i18n/locale";
import { connectionStateText } from "../../i18n/connections";
import { serverOverviewMessages } from "../../i18n/server-overview";
import BaseDropdownMenu from "../base/BaseDropdownMenu.vue";
import BaseIcon from "../base/BaseIcon.vue";
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
const endpointLabel = computed(() => {
  const endpoint = formatServerEndpoint(props.server.username, props.server.host, props.server.port);
  const endpointWithoutPort = endpoint.slice(0, endpoint.lastIndexOf(":"));
  if (props.server.name !== endpointWithoutPort) return endpoint;
  const authentication = t(props.server.authType === "privateKey" ? "privateKeyAuthentication" : "passwordAuthentication");
  return t("serverConnectionMetadata", { port: props.server.port, authentication });
});
</script>

<template>
  <header class="server-overview-header">
    <nav class="server-overview-breadcrumb" :aria-label="t('serverBreadcrumbNavigation')">
      <button class="server-overview-breadcrumb-back" type="button" @click="emit('back')">
        <BaseIcon name="arrow-left" />
        <span>{{ t('backToServers') }}</span>
      </button>
      <span class="server-overview-breadcrumb-separator" aria-hidden="true">/</span>
      <span class="server-overview-breadcrumb-current" aria-current="page">{{ t('serverDetails') }}</span>
    </nav>
    <div class="server-overview-identity-actions">
      <div class="server-overview-identity">
        <div class="server-overview-title-line">
          <h1>{{ server.name }}</h1>
          <BaseStatusBadge :tone="statusTone()">{{ statusText() }}</BaseStatusBadge>
        </div>
        <p class="server-overview-endpoint">{{ endpointLabel }}</p>
      </div>
      <div class="server-overview-actions">
        <slot name="connection-actions" />
        <BaseDropdownMenu :label="`${t('moreActions')} · ${server.name}`" :items="menuItems" @action="emit('menuAction', $event)" />
      </div>
    </div>
  </header>
</template>
