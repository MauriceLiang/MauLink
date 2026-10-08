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
import type { createServerRuntimeStatsApi } from "../../ipc/server-runtime-stats";
import RecentActivityCard from "./RecentActivityCard.vue";

const props = defineProps<{ server: ServerProfile; store: ConnectionStore; hostKeyApi: ReturnType<typeof createHostKeysApi>; networkApi: ReturnType<typeof createNetworkApi>; preflightApi: ReturnType<typeof createPreflightApi>; runtimeStatsApi: ReturnType<typeof createServerRuntimeStatsApi>; readOnly: boolean; databaseRevision?: number }>();
const emit = defineEmits<{ back: []; edit: [id: string]; remove: [server: ServerProfile]; copy: [kind: "address" | "ssh", value: string]; openSettings: []; testResult: [result: { kind: "success" | "error"; message: string }] }>();
const t = messages(serverOverviewMessages);
const overviewSections = ["connection", "network", "security", "route", "activity"] as const;
type OverviewSection = (typeof overviewSections)[number];
type NetworkSummary = { serverId: string; host: string; port: number; status: "loading" } | { serverId: string; host: string; port: number; status: "error" } | { serverId: string; host: string; port: number; status: "ready"; inspection: NetworkInspection };
type TrustSummary = { serverId: string; host: string; port: number; status: "loading" | "empty" | "error" | "available" };
const sectionTitleKeys = {
  connection: "connectionConfiguration",
  network: "networkInformation",
  security: "securityIdentity",
  route: "connectionRoute",
  activity: "recentActivity",
} as const satisfies Record<OverviewSection, keyof typeof serverOverviewMessages>;
const overviewId = useId();
const activeSection = ref<OverviewSection>("connection");
const networkSummary = ref<NetworkSummary>({ serverId: props.server.id, host: props.server.host, port: props.server.port, status: "loading" });
const trustSummary = ref<TrustSummary>({ serverId: props.server.id, host: props.server.host, port: props.server.port, status: "loading" });
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const runtimeStats = ref<ServerRuntimeStats | null>(null);
const runtimeStatsState = ref<"loading" | "ready" | "error">("loading");
const sshCommand = computed(() => buildSshCommand(props.server));
const lastPreflightSummary = computed(() => {
  if (runtimeStatsState.value === "loading") return t("preflightSummaryLoading");
  if (runtimeStatsState.value === "error") return t("preflightSummaryUnavailable");
  const stats = runtimeStats.value;
  if (!stats || stats.serverId !== props.server.id || stats.lastPreflightAtMs === null) return t("noRecentPreflight");
  const checkedAt = new Intl.DateTimeFormat(locale.value === "en" ? "en-US" : "zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(stats.lastPreflightAtMs));
  const latency = stats.lastPreflightLatencyMs === null ? null : `${stats.lastPreflightLatencyMs} ${t("milliseconds")}`;
  return [latency, checkedAt].filter(Boolean).join(" · ");
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
function sectionTitle(section: OverviewSection) { return t(sectionTitleKeys[section]); }
function selectSection(section: OverviewSection) { activeSection.value = section; }
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
    <dl class="server-overview-status-strip" :aria-label="t('overviewSummary')">
      <div class="server-overview-status-item" data-summary="network">
        <dt>{{ t('networkInformation') }}</dt>
        <dd>{{ networkSummaryText }}</dd>
      </div>
      <div class="server-overview-status-item" data-summary="trust">
        <dt>{{ t('hostTrust') }}</dt>
        <dd>{{ trustSummaryText }}</dd>
      </div>
      <div class="server-overview-status-item" data-summary="preflight">
        <dt>{{ t('lastPreflightStatus') }}</dt>
        <dd>{{ lastPreflightSummary }}</dd>
      </div>
    </dl>
    <div class="server-overview-workbench">
      <nav class="server-overview-tabs" :aria-label="t('overviewSectionNavigation')">
        <button
          v-for="section in overviewSections"
          :id="sectionButtonId(section)"
          :key="section"
          class="server-overview-tab"
          :class="{ 'is-active': activeSection === section }"
          :data-section="section"
          type="button"
          :aria-current="activeSection === section ? 'true' : undefined"
          :aria-controls="sectionPanelId(section)"
          @click="selectSection(section)"
        >
          {{ sectionTitle(section) }}
        </button>
      </nav>
      <div class="server-overview-detail-content">
        <h2 class="server-overview-detail-heading">{{ sectionTitle(activeSection) }}</h2>
        <div :id="sectionPanelId('connection')" v-show="activeSection === 'connection'" class="server-overview-section-panel" role="region" :aria-labelledby="sectionButtonId('connection')">
          <ConnectionInfoCard :server="server" embedded />
        </div>
        <div :id="sectionPanelId('network')" v-show="activeSection === 'network'" class="server-overview-section-panel" role="region" :aria-labelledby="sectionButtonId('network')">
          <NetworkInfoCard :server="server" :api="networkApi" :database-revision="databaseRevision" embedded @open-settings="emit('openSettings')" @summary-change="onNetworkSummary" />
        </div>
        <div :id="sectionPanelId('security')" v-show="activeSection === 'security'" class="server-overview-section-panel" role="region" :aria-labelledby="sectionButtonId('security')">
          <HostIdentityCard :server="server" :api="hostKeyApi" embedded @summary-change="onTrustSummary" />
        </div>
        <div :id="sectionPanelId('route')" v-show="activeSection === 'route'" class="server-overview-section-panel" role="region" :aria-labelledby="sectionButtonId('route')">
          <ConnectionRouteCard :server="server" embedded />
        </div>
        <div :id="sectionPanelId('activity')" v-show="activeSection === 'activity'" class="server-overview-section-panel" role="region" :aria-labelledby="sectionButtonId('activity')">
          <RecentActivityCard :stats="runtimeStats" :snapshot="snapshot" :state="runtimeStatsState" embedded />
        </div>
      </div>
    </div>
  </section>
</template>
