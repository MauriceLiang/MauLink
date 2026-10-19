<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { NetworkInspection } from "../../../../contracts/v1/NetworkInspection";
import type { NetworkScope } from "../../../../contracts/v1/NetworkScope";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";
import type { createNetworkApi } from "../../ipc/network";
import { mapError } from "../../errors/mapper";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";

const props = withDefaults(defineProps<{ server: ServerProfile; api: ReturnType<typeof createNetworkApi>; embedded?: boolean }>(), { embedded: false });
const emit = defineEmits<{
  summaryChange: [summary: { serverId: string; host: string; port: number; status: "loading" } | { serverId: string; host: string; port: number; status: "error" } | { serverId: string; host: string; port: number; status: "ready"; inspection: NetworkInspection }];
}>();
type LoadState = { status: "loading" | "error"; detailed: boolean } | { status: "ready"; detailed: boolean; inspection: NetworkInspection };
const t = messages(serverOverviewMessages);
const state = ref<LoadState>({ status: "loading", detailed: true });
const readyState = computed(() => state.value.status === "ready" ? state.value : null);
const errorMessage = ref("");
let requestVersion = 0;

function setState(next: LoadState, server: ServerProfile) {
  state.value = next;
  emit("summaryChange", next.status === "ready"
    ? { serverId: server.id, host: server.host, port: server.port, status: "ready", inspection: next.inspection }
    : { serverId: server.id, host: server.host, port: server.port, status: next.status });
}

async function load(detailed: boolean) {
  const server = props.server;
  const version = ++requestVersion;
  errorMessage.value = "";
  setState({ status: "loading", detailed }, server);
  try {
    const inspection = await props.api.inspect({ host: server.host, detailed });
    if (version === requestVersion) setState({ status: "ready", detailed, inspection }, server);
  } catch (reason) {
    if (version === requestVersion) { errorMessage.value = presentError(mapError(reason)).message; setState({ status: "error", detailed }, server); }
  }
}

const typeLabel = computed(() => {
  if (!readyState.value) return "";
  const { ipVersion, scope, hostKind } = readyState.value.inspection;
  if (!ipVersion || !scope) return hostKind === "hostname" ? t("networkHostname") : "—";
  return `${t(ipVersion === "ipv4" ? "networkIpVersion4" : "networkIpVersion6")} · ${t(scopeKey(scope))}`;
});
const scopeKeys = {
  public: "networkScopePublic",
  private: "networkScopePrivate",
  loopback: "networkScopeLoopback",
  linkLocal: "networkScopeLinkLocal",
  unspecified: "networkScopeUnspecified",
  multicast: "networkScopeMulticast",
  reserved: "networkScopeReserved",
} as const satisfies Record<NetworkScope, keyof typeof serverOverviewMessages>;

function scopeKey(scope: NetworkScope): keyof typeof serverOverviewMessages {
  return scopeKeys[scope];
}

watch(() => [props.server.id, props.server.host, props.server.port] as const, () => { void load(true); }, { immediate: true });
onBeforeUnmount(() => { requestVersion += 1; });
</script>

<template>
  <section class="server-overview-card server-overview-network" :class="{ 'server-overview-card--embedded': embedded }" :aria-label="t('networkInformation')">
    <h2 v-if="!embedded">{{ t('networkInformation') }}</h2>
    <div class="server-overview-network-state" :aria-live="state.status === 'loading' ? 'polite' : 'off'">
      <p v-if="state.status === 'loading'">{{ t('networkAnalysisLoading') }}</p>
      <template v-else-if="state.status === 'error'">
        <p role="alert">{{ errorMessage || t('networkAnalysisFailed') }}</p>
        <BaseButton @click="load(state.detailed)">{{ t('networkRetry') }}</BaseButton>
      </template>
      <template v-else-if="readyState">
        <div class="server-overview-network-summary">
          <strong v-if="readyState?.inspection.primaryAddress" class="server-overview-mono">{{ readyState.inspection.primaryAddress }}</strong>
          <strong v-else>{{ t('networkHostname') }}</strong>
          <span v-if="typeLabel">{{ typeLabel }}</span>
        </div>
        <details v-if="readyState.detailed" class="server-overview-extra server-overview-network-extra">
          <summary>{{ t('networkTechnicalDetails') }}</summary>
          <dl class="server-overview-info-grid">
            <div class="server-overview-info-item"><dt>{{ t('networkResolvedAddresses') }}</dt><dd class="server-overview-mono">{{ readyState.inspection.resolvedAddresses.join(', ') || '—' }}</dd></div>
            <div v-if="readyState.inspection.reverseDns" class="server-overview-info-item"><dt>{{ t('networkReverseDns') }}</dt><dd class="server-overview-mono">{{ readyState.inspection.reverseDns }}</dd></div>
            <div class="server-overview-info-item"><dt>{{ t('networkSource') }}</dt><dd>{{ readyState.inspection.source === 'localAnalysis' ? t('networkLocalAnalysis') : t('networkSystemResolver') }}</dd></div>
          </dl>
        </details>
      </template>
    </div>
  </section>
</template>
