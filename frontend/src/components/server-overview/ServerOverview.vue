<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId, watch } from "vue";
import type { NetworkInspection } from "../../../../contracts/v1/NetworkInspection";
import type { NetworkScope } from "../../../../contracts/v1/NetworkScope";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import type { ServerRuntimeStats } from "../../../../contracts/v1/ServerRuntimeStats";
import { locale, messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";
import type { ConnectionStore } from "../../stores/connections";
import type { MenuItem } from "../base/menu";
import BaseButton from "../base/BaseButton.vue";
import BaseIcon from "../base/BaseIcon.vue";
import ConnectionPanel from "../connection/ConnectionPanel.vue";
import ConnectionInfoCard from "./ConnectionInfoCard.vue";
import ConnectionRouteCard from "./ConnectionRouteCard.vue";
import HostIdentityCard from "./HostIdentityCard.vue";
import NetworkInfoCard from "./NetworkInfoCard.vue";
import ServerOverviewHeader from "./ServerOverviewHeader.vue";
import { buildSshCommand, connectionRoute, formatServerEndpoint } from "./server-details";
import type { createHostKeysApi } from "../../ipc/host-keys";
import type { createNetworkApi } from "../../ipc/network";
import type { createPreflightApi } from "../../ipc/preflight";
import type { createServerRuntimeStatsApi } from "../../ipc/server-runtime-stats";
import RecentActivityCard from "./RecentActivityCard.vue";

const props = defineProps<{ server: ServerProfile; store: ConnectionStore; hostKeyApi: ReturnType<typeof createHostKeysApi>; networkApi: ReturnType<typeof createNetworkApi>; preflightApi: ReturnType<typeof createPreflightApi>; runtimeStatsApi: ReturnType<typeof createServerRuntimeStatsApi>; readOnly: boolean; databaseRevision?: number }>();
const emit = defineEmits<{ back: []; edit: [id: string]; remove: [server: ServerProfile]; copy: [kind: "address" | "ssh", value: string]; openSettings: []; testResult: [result: { kind: "success" | "error"; message: string }] }>();
const t = messages(serverOverviewMessages);
type OverviewSection = "connection" | "network" | "security" | "route" | "activity";
type NetworkSummary = { serverId: string; host: string; port: number; status: "loading" } | { serverId: string; host: string; port: number; status: "error" } | { serverId: string; host: string; port: number; status: "ready"; inspection: NetworkInspection };
type TrustSummary = { serverId: string; host: string; port: number; status: "loading" | "empty" | "error" | "available" };
const overviewId = useId();
const activeSection = ref<OverviewSection | null>("connection");
const networkSummary = ref<NetworkSummary>({ serverId: props.server.id, host: props.server.host, port: props.server.port, status: "loading" });
const trustSummary = ref<TrustSummary>({ serverId: props.server.id, host: props.server.host, port: props.server.port, status: "loading" });
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const runtimeStats = ref<ServerRuntimeStats | null>(null);
const runtimeStatsState = ref<"loading" | "ready" | "error">("loading");
const sshCommand = computed(() => buildSshCommand(props.server));
const authenticationSummary = computed(() => `${props.server.username} · ${t(props.server.authType === "privateKey" ? "privateKey" : "password")} · ${props.server.port}`);
const scopeKeys = {
  public: "networkScopePublic",
  private: "networkScopePrivate",
  loopback: "networkScopeLoopback",
  linkLocal: "networkScopeLinkLocal",
  unspecified: "networkScopeUnspecified",
  multicast: "networkScopeMulticast",
  reserved: "networkScopeReserved",
} as const satisfies Record<NetworkScope, keyof typeof serverOverviewMessages>;
const networkSummaryText = computed(() => {
  const result = networkSummary.value;
  if (result.serverId !== props.server.id || result.host !== props.server.host || result.port !== props.server.port) return t("networkSummaryLoading");
  if (result.status === "loading") return t("networkSummaryLoading");
  if (result.status === "error") return t("networkSummaryError");
  const inspection = result.inspection;
  const location = [inspection.geo.city, inspection.geo.region, inspection.geo.countryName || inspection.geo.countryCode].filter((value): value is string => Boolean(value)).join(" · ");
  const type = inspection.ipVersion && inspection.scope
    ? `${t(inspection.ipVersion === "ipv4" ? "networkIpVersion4" : "networkIpVersion6")} · ${t(scopeKeys[inspection.scope])}`
    : inspection.hostKind === "hostname" ? t("networkHostname") : inspection.primaryAddress ?? t("networkSummaryUnknown");
  return location || inspection.organization || type;
});
const trustSummaryText = computed(() => {
  const result = trustSummary.value;
  if (result.serverId !== props.server.id || result.host !== props.server.host || result.port !== props.server.port || result.status === "loading") return t("trustSummaryLoading");
  if (result.status === "available") return t("trustSummaryAvailable");
  if (result.status === "empty") return t("trustSummaryEmpty");
  return t("trustSummaryError");
});
const routeSteps = computed(() => connectionRoute(props.server, {
  local: t("localMachine"), proxy: t("proxy"), jumpHost: t("jumpHost"), server: t("server"),
  socks5: t("proxySocks5"), httpConnect: t("proxyHttpConnect"),
}));
const routeSummary = computed(() => routeSteps.value.map(step => step.label).join(" → "));
function formatSummaryDate(timestamp: number) {
  return new Intl.DateTimeFormat(locale.value === "en" ? "en-US" : "zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp));
}
const activitySummary = computed(() => {
  const current = snapshot.value?.mode === "workspace" ? snapshot.value : null;
  const candidates: Array<{ kind: "success" | "failure" | "preflight"; timestamp: number; detail?: string }> = [];
  const lastSuccessAtMs = current?.state === "ready" && current.updatedAtMs > (runtimeStats.value?.lastSuccessAtMs ?? 0)
    ? current.updatedAtMs : runtimeStats.value?.lastSuccessAtMs ?? null;
  const lastFailureAtMs = current?.state === "failed" && current.updatedAtMs > (runtimeStats.value?.lastFailureAtMs ?? 0)
    ? current.updatedAtMs : runtimeStats.value?.lastFailureAtMs ?? null;
  const lastFailureCode = current?.state === "failed" && current.updatedAtMs === lastFailureAtMs
    ? current.error?.code ?? null : runtimeStats.value?.lastFailureCode ?? null;
  if (lastSuccessAtMs !== null) candidates.push({ kind: "success", timestamp: lastSuccessAtMs });
  if (lastFailureAtMs !== null) candidates.push({ kind: "failure", timestamp: lastFailureAtMs, detail: lastFailureCode ?? undefined });
  if (runtimeStats.value?.lastPreflightAtMs !== null && runtimeStats.value?.lastPreflightAtMs !== undefined) candidates.push({ kind: "preflight", timestamp: runtimeStats.value.lastPreflightAtMs, detail: runtimeStats.value.lastPreflightLatencyMs === null ? undefined : `${runtimeStats.value.lastPreflightLatencyMs} ${t("milliseconds")}` });
  const latest = candidates.sort((a, b) => b.timestamp - a.timestamp)[0];
  if (latest) {
    const label = t(latest.kind === "success" ? "lastSuccessfulConnection" : latest.kind === "failure" ? "lastFailedConnection" : "lastPreflight");
    return [label, latest.detail, formatSummaryDate(latest.timestamp)].filter(Boolean).join(" · ");
  }
  if (runtimeStatsState.value === "loading") return t("activitySummaryLoading");
  if (runtimeStatsState.value === "error") return t("activityUnavailable");
  return t("noRecentActivity");
});
function toggleSection(section: OverviewSection) {
  activeSection.value = activeSection.value === section ? null : section;
}
function sectionButtonId(section: OverviewSection) { return `${overviewId}-${section}-button`; }
function sectionPanelId(section: OverviewSection) { return `${overviewId}-${section}-panel`; }
function isCurrentEndpoint(endpoint: { serverId: string; host: string; port: number }) {
  return endpoint.serverId === props.server.id && endpoint.host === props.server.host && endpoint.port === props.server.port;
}
function onNetworkSummary(summary: NetworkSummary) {
  if (isCurrentEndpoint(summary)) networkSummary.value = summary;
}
function onTrustSummary(summary: TrustSummary) {
  if (isCurrentEndpoint(summary)) trustSummary.value = summary;
}
const preflightState = ref<"idle" | "loading">("idle");
let preflightRequestVersion = 0;
let runtimeStatsRequestVersion = 0;
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
  preflightState.value = "loading";
  try {
    const result = await props.preflightApi.check({
      serverId: props.server.id,
      host: props.server.host,
      port: props.server.port,
      timeoutMs: props.server.connectTimeoutMs,
    });
    if (requestVersion === preflightRequestVersion) {
      preflightState.value = "idle";
      const success = result.tcpReachable === true && result.error === null;
      emit("testResult", { kind: success ? "success" : "error", message: t(success ? "preflightSuccess" : "preflightFailure", { latency: result.tcpConnectDurationMs ?? "—" }) });
      await loadRuntimeStats();
    }
  } catch {
    if (requestVersion === preflightRequestVersion) {
      preflightState.value = "idle";
      emit("testResult", { kind: "error", message: t("preflightFailure", { latency: "—" }) });
    }
  }
}

async function loadRuntimeStats() {
  const requestVersion = ++runtimeStatsRequestVersion;
  runtimeStatsState.value = "loading";
  try {
    const stats = await props.runtimeStatsApi.get({ serverId: props.server.id });
    if (requestVersion === runtimeStatsRequestVersion) {
      runtimeStats.value = stats;
      runtimeStatsState.value = "ready";
    }
  } catch {
    if (requestVersion === runtimeStatsRequestVersion) {
      runtimeStats.value = null;
      runtimeStatsState.value = "error";
    }
  }
}

watch(() => [props.server.id, props.server.host, props.server.port, props.server.connectTimeoutMs], () => {
  preflightRequestVersion += 1;
  preflightState.value = "idle";
});
watch(() => [props.server.id, props.server.host, props.server.port] as const, ([serverId, host, port], previous) => {
  if (previous && serverId !== previous[0]) activeSection.value = "connection";
  networkSummary.value = { serverId, host, port, status: "loading" };
  trustSummary.value = { serverId, host, port, status: "loading" };
});
watch(() => props.server.id, () => { runtimeStats.value = null; void loadRuntimeStats(); }, { immediate: true });
watch(() => snapshot.value?.state, state => {
  if (state === "ready" || state === "failed") void loadRuntimeStats();
});
onBeforeUnmount(() => { preflightRequestVersion += 1; runtimeStatsRequestVersion += 1; });
</script>

<template>
  <section class="server-overview">
    <ServerOverviewHeader :server="server" :state="snapshot?.state" :menu-items="menuItems" @back="emit('back')" @menu-action="menuAction">
      <template #connection-actions>
        <ConnectionPanel :server="server" :store="store" :read-only="readOnly" :connect-label="t('connect')">
          <BaseButton :loading="preflightState === 'loading'" @click="runPreflight">{{ preflightState === 'loading' ? t('connectionCheckLoading') : t('connectionCheck') }}</BaseButton>
        </ConnectionPanel>
      </template>
    </ServerOverviewHeader>
    <dl class="server-overview-summary" :aria-label="t('overviewSummary')">
      <div class="server-overview-summary-item">
        <dt>{{ t('authentication') }}</dt>
        <dd>{{ authenticationSummary }}</dd>
      </div>
      <div class="server-overview-summary-item">
        <dt>{{ t('networkInformation') }}</dt>
        <dd>{{ networkSummaryText }}</dd>
      </div>
      <div class="server-overview-summary-item">
        <dt>{{ t('securityIdentity') }}</dt>
        <dd>{{ trustSummaryText }}</dd>
      </div>
    </dl>
    <h2 class="server-overview-details-heading">{{ t('overviewDetails') }}</h2>
    <div class="server-overview-accordion">
      <section class="server-overview-accordion-item">
        <h3 class="server-overview-accordion-heading">
          <button :id="sectionButtonId('connection')" class="server-overview-accordion-trigger" type="button" :aria-expanded="activeSection === 'connection'" :aria-controls="sectionPanelId('connection')" @click="toggleSection('connection')">
            <span class="server-overview-accordion-copy"><span class="server-overview-accordion-title">{{ t('connectionInfo') }}</span><span class="server-overview-accordion-summary">{{ authenticationSummary }}</span></span>
            <span class="server-overview-accordion-chevron" :class="{ 'is-expanded': activeSection === 'connection' }" aria-hidden="true"><BaseIcon name="chevron-down" /></span>
          </button>
        </h3>
        <div :id="sectionPanelId('connection')" v-show="activeSection === 'connection'" class="server-overview-accordion-panel" role="region" :aria-labelledby="sectionButtonId('connection')">
          <ConnectionInfoCard :server="server" embedded />
        </div>
      </section>
      <section class="server-overview-accordion-item">
        <h3 class="server-overview-accordion-heading">
          <button :id="sectionButtonId('network')" class="server-overview-accordion-trigger" type="button" :aria-expanded="activeSection === 'network'" :aria-controls="sectionPanelId('network')" @click="toggleSection('network')">
            <span class="server-overview-accordion-copy"><span class="server-overview-accordion-title">{{ t('networkInformation') }}</span><span class="server-overview-accordion-summary">{{ networkSummaryText }}</span></span>
            <span class="server-overview-accordion-chevron" :class="{ 'is-expanded': activeSection === 'network' }" aria-hidden="true"><BaseIcon name="chevron-down" /></span>
          </button>
        </h3>
        <div :id="sectionPanelId('network')" v-show="activeSection === 'network'" class="server-overview-accordion-panel" role="region" :aria-labelledby="sectionButtonId('network')">
          <NetworkInfoCard :server="server" :api="networkApi" :database-revision="databaseRevision" embedded @open-settings="emit('openSettings')" @summary-change="onNetworkSummary" />
        </div>
      </section>
      <section class="server-overview-accordion-item">
        <h3 class="server-overview-accordion-heading">
          <button :id="sectionButtonId('security')" class="server-overview-accordion-trigger" type="button" :aria-expanded="activeSection === 'security'" :aria-controls="sectionPanelId('security')" @click="toggleSection('security')">
            <span class="server-overview-accordion-copy"><span class="server-overview-accordion-title">{{ t('securityIdentity') }}</span><span class="server-overview-accordion-summary">{{ trustSummaryText }}</span></span>
            <span class="server-overview-accordion-chevron" :class="{ 'is-expanded': activeSection === 'security' }" aria-hidden="true"><BaseIcon name="chevron-down" /></span>
          </button>
        </h3>
        <div :id="sectionPanelId('security')" v-show="activeSection === 'security'" class="server-overview-accordion-panel" role="region" :aria-labelledby="sectionButtonId('security')">
          <HostIdentityCard :server="server" :api="hostKeyApi" embedded @summary-change="onTrustSummary" />
        </div>
      </section>
      <section class="server-overview-accordion-item">
        <h3 class="server-overview-accordion-heading">
          <button :id="sectionButtonId('route')" class="server-overview-accordion-trigger" type="button" :aria-expanded="activeSection === 'route'" :aria-controls="sectionPanelId('route')" @click="toggleSection('route')">
            <span class="server-overview-accordion-copy"><span class="server-overview-accordion-title">{{ t('connectionRoute') }}</span><span class="server-overview-accordion-summary">{{ routeSummary }}</span></span>
            <span class="server-overview-accordion-chevron" :class="{ 'is-expanded': activeSection === 'route' }" aria-hidden="true"><BaseIcon name="chevron-down" /></span>
          </button>
        </h3>
        <div :id="sectionPanelId('route')" v-show="activeSection === 'route'" class="server-overview-accordion-panel" role="region" :aria-labelledby="sectionButtonId('route')">
          <ConnectionRouteCard :server="server" embedded />
        </div>
      </section>
      <section class="server-overview-accordion-item">
        <h3 class="server-overview-accordion-heading">
          <button :id="sectionButtonId('activity')" class="server-overview-accordion-trigger" type="button" :aria-expanded="activeSection === 'activity'" :aria-controls="sectionPanelId('activity')" @click="toggleSection('activity')">
            <span class="server-overview-accordion-copy"><span class="server-overview-accordion-title">{{ t('recentActivity') }}</span><span class="server-overview-accordion-summary">{{ activitySummary }}</span></span>
            <span class="server-overview-accordion-chevron" :class="{ 'is-expanded': activeSection === 'activity' }" aria-hidden="true"><BaseIcon name="chevron-down" /></span>
          </button>
        </h3>
        <div :id="sectionPanelId('activity')" v-show="activeSection === 'activity'" class="server-overview-accordion-panel" role="region" :aria-labelledby="sectionButtonId('activity')">
          <RecentActivityCard :stats="runtimeStats" :snapshot="snapshot" :state="runtimeStatsState" embedded />
        </div>
      </section>
    </div>
  </section>
</template>
