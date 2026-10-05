<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { ConnectionPreflightError } from "../../../../contracts/v1/ConnectionPreflightError";
import type { ConnectionPreflightResult } from "../../../../contracts/v1/ConnectionPreflightResult";
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
import NetworkInfoCard from "./NetworkInfoCard.vue";
import ServerOverviewHeader from "./ServerOverviewHeader.vue";
import { buildSshCommand, formatServerEndpoint } from "./server-details";
import type { createHostKeysApi } from "../../ipc/host-keys";
import type { createNetworkApi } from "../../ipc/network";
import type { createPreflightApi } from "../../ipc/preflight";

const props = defineProps<{ server: ServerProfile; store: ConnectionStore; hostKeyApi: ReturnType<typeof createHostKeysApi>; networkApi: ReturnType<typeof createNetworkApi>; preflightApi: ReturnType<typeof createPreflightApi>; readOnly: boolean }>();
const emit = defineEmits<{ back: []; edit: [id: string]; remove: [server: ServerProfile]; copy: [kind: "address" | "ssh", value: string] }>();
const t = messages(serverOverviewMessages);
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const sshCommand = computed(() => buildSshCommand(props.server));
type PreflightState = { status: "idle" | "loading" | "requestError" } | { status: "ready"; result: ConnectionPreflightResult };
const preflightState = ref<PreflightState>({ status: "idle" });
const preflightErrorKeys = {
  dnsFailed: "preflightDnsFailed",
  timeout: "preflightTimeout",
  connectionRefused: "preflightConnectionRefused",
  connectionFailed: "preflightConnectionFailed",
} as const satisfies Record<ConnectionPreflightError, keyof typeof serverOverviewMessages>;
let preflightRequestVersion = 0;
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

async function runPreflight() {
  const requestVersion = ++preflightRequestVersion;
  preflightState.value = { status: "loading" };
  try {
    const result = await props.preflightApi.check({
      host: props.server.host,
      port: props.server.port,
      timeoutMs: props.server.connectTimeoutMs,
    });
    if (requestVersion === preflightRequestVersion) preflightState.value = { status: "ready", result };
  } catch {
    if (requestVersion === preflightRequestVersion) preflightState.value = { status: "requestError" };
  }
}

watch(() => [props.server.id, props.server.host, props.server.port, props.server.connectTimeoutMs], () => {
  preflightRequestVersion += 1;
  preflightState.value = { status: "idle" };
});
onBeforeUnmount(() => { preflightRequestVersion += 1; });
</script>

<template>
  <section class="server-overview">
    <ServerOverviewHeader :server="server" :state="snapshot?.state" :menu-items="menuItems" @back="emit('back')" @menu-action="menuAction">
      <template #connection-actions>
        <ConnectionPanel :server="server" :store="store" :read-only="readOnly" :connect-label="t('connect')">
          <BaseButton :loading="preflightState.status === 'loading'" @click="runPreflight">{{ preflightState.status === 'loading' ? t('connectionCheckLoading') : t('connectionCheck') }}</BaseButton>
        </ConnectionPanel>
      </template>
    </ServerOverviewHeader>
    <section v-if="preflightState.status === 'ready'" class="server-overview-card server-overview-preflight" :aria-label="t('connectionCheckResult')" role="status">
      <h2>{{ t('connectionCheckResult') }}</h2>
      <dl class="server-overview-info-grid">
        <div class="server-overview-info-item"><dt>{{ t('preflightResolvedAddresses') }}</dt><dd class="server-overview-mono">{{ preflightState.result.resolvedAddresses.join(', ') || '—' }}</dd></div>
        <div class="server-overview-info-item"><dt>{{ t('preflightSelectedAddress') }}</dt><dd class="server-overview-mono">{{ preflightState.result.selectedAddress || '—' }}</dd></div>
        <div class="server-overview-info-item"><dt>{{ t('preflightDnsDuration') }}</dt><dd>{{ preflightState.result.dnsDurationMs }} {{ t('milliseconds') }}</dd></div>
        <div class="server-overview-info-item"><dt>{{ t('preflightTcpPort', { port: server.port }) }}</dt><dd>{{ preflightState.result.tcpReachable === true ? t('preflightTcpReachable') : preflightState.result.tcpReachable === false ? t('preflightTcpUnreachable') : t('preflightTcpNotAttempted') }}</dd></div>
        <div v-if="preflightState.result.tcpConnectDurationMs !== null" class="server-overview-info-item"><dt>{{ t('preflightTcpDuration') }}</dt><dd>{{ preflightState.result.tcpConnectDurationMs }} {{ t('milliseconds') }}</dd></div>
      </dl>
      <p v-if="preflightState.result.error" class="server-overview-preflight-error" role="alert">{{ t(preflightErrorKeys[preflightState.result.error], { timeout: server.connectTimeoutMs }) }}</p>
      <p class="server-overview-preflight-note">{{ t('preflightSshNote') }}</p>
    </section>
    <p v-else-if="preflightState.status === 'requestError'" class="server-overview-preflight-error" role="alert">{{ t('preflightRequestFailed') }}</p>
    <div class="server-overview-grid">
      <ConnectionInfoCard :server="server" />
      <NetworkInfoCard :server="server" :api="networkApi" />
      <HostIdentityCard :server="server" :api="hostKeyApi" />
      <ConnectionRouteCard :server="server" />
    </div>
  </section>
</template>
