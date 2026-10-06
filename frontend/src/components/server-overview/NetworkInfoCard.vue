<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { NetworkInspection } from "../../../../contracts/v1/NetworkInspection";
import type { NetworkScope } from "../../../../contracts/v1/NetworkScope";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { locale, messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";
import type { createNetworkApi } from "../../ipc/network";
import BaseButton from "../base/BaseButton.vue";

const props = defineProps<{ server: ServerProfile; api: ReturnType<typeof createNetworkApi>; databaseRevision?: number }>();
const emit = defineEmits<{ openSettings: [] }>();
type LoadState = { status: "loading" | "error"; detailed: boolean } | { status: "ready"; detailed: boolean; inspection: NetworkInspection };
const t = messages(serverOverviewMessages);
const state = ref<LoadState>({ status: "loading", detailed: false });
const readyState = computed(() => state.value.status === "ready" ? state.value : null);
let requestVersion = 0;

async function load(detailed: boolean) {
  const version = ++requestVersion;
  state.value = { status: "loading", detailed };
  try {
    const inspection = await props.api.inspect({ host: props.server.host, detailed });
    if (version === requestVersion) state.value = { status: "ready", detailed, inspection };
  } catch {
    if (version === requestVersion) state.value = { status: "error", detailed };
  }
}

const typeLabel = computed(() => {
  if (!readyState.value) return "";
  const { ipVersion, scope, hostKind } = readyState.value.inspection;
  if (!ipVersion || !scope) return hostKind === "hostname" ? t("networkHostname") : "—";
  return `${t(ipVersion === "ipv4" ? "networkIpVersion4" : "networkIpVersion6")} · ${t(scopeKey(scope))}`;
});
const geoLabel = computed(() => {
  if (!readyState.value) return "—";
  const { countryName, countryCode, region, city } = readyState.value.inspection.geo;
  return [countryName || countryCode, region, city].filter(Boolean).join(" · ") || "—";
});
const hasGeoData = computed(() => !!readyState.value && !!(
  readyState.value.inspection.geo.countryCode || readyState.value.inspection.geo.countryName
  || readyState.value.inspection.geo.region || readyState.value.inspection.geo.city
  || readyState.value.inspection.asn || readyState.value.inspection.organization
));

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

function formatDate(timestamp: number) {
  return new Intl.DateTimeFormat(locale.value === "en" ? "en-US" : "zh-CN", { dateStyle: "medium" }).format(new Date(timestamp));
}

watch(() => props.server.host, () => { void load(false); }, { immediate: true });
watch(() => props.databaseRevision, () => { void load(state.value.detailed); });
onBeforeUnmount(() => { requestVersion += 1; });
</script>

<template>
  <section class="server-overview-card server-overview-network" :aria-label="t('networkInformation')">
    <h2>{{ t('networkInformation') }}</h2>
    <p class="server-overview-network-note">{{ t('networkPrivacyNote') }}</p>
    <div class="server-overview-network-state" :aria-live="state.status === 'loading' ? 'polite' : 'off'">
      <p v-if="state.status === 'loading'">{{ t('networkAnalysisLoading') }}</p>
      <template v-else-if="state.status === 'error'">
        <p role="alert">{{ t('networkAnalysisFailed') }}</p>
        <BaseButton @click="load(state.detailed)">{{ t('networkRetry') }}</BaseButton>
      </template>
      <template v-else-if="readyState">
        <div class="server-overview-network-summary">
          <strong v-if="readyState?.inspection.primaryAddress" class="server-overview-mono">{{ readyState.inspection.primaryAddress }}</strong>
          <strong v-else>{{ t('networkHostname') }}</strong>
          <span v-if="typeLabel">{{ typeLabel }}</span>
        </div>
        <dl v-if="readyState.detailed && hasGeoData" class="server-overview-info-grid">
          <div v-if="geoLabel !== '—'" class="server-overview-info-item"><dt>{{ t('networkCountryRegion') }}</dt><dd>{{ geoLabel }}</dd></div>
          <div v-if="readyState.inspection.asn" class="server-overview-info-item"><dt>{{ t('networkAsn') }}</dt><dd>{{ readyState.inspection.asn }}</dd></div>
          <div v-if="readyState.inspection.organization" class="server-overview-info-item"><dt>{{ t('networkOrganization') }}</dt><dd>{{ readyState.inspection.organization }}</dd></div>
        </dl>
        <p v-if="readyState.detailed && readyState.inspection.scope !== 'public'" class="server-overview-network-hint">{{ t('networkPrivateNoGeo') }}</p>
        <div v-else-if="readyState.detailed && !hasGeoData" class="server-overview-network-configure">
          <span class="server-overview-network-hint">{{ t('networkGeoUnavailable') }}</span>
          <BaseButton @click="emit('openSettings')">{{ t('configureGeoDatabase') }}</BaseButton>
        </div>
        <details v-if="readyState.detailed" class="server-overview-extra server-overview-network-extra">
          <summary>{{ t('networkTechnicalDetails') }}</summary>
          <dl class="server-overview-info-grid">
            <div class="server-overview-info-item"><dt>{{ t('networkResolvedAddresses') }}</dt><dd class="server-overview-mono">{{ readyState.inspection.resolvedAddresses.join(', ') || '—' }}</dd></div>
            <div v-if="readyState.inspection.reverseDns" class="server-overview-info-item"><dt>{{ t('networkReverseDns') }}</dt><dd class="server-overview-mono">{{ readyState.inspection.reverseDns }}</dd></div>
            <div v-if="readyState.inspection.databaseUpdatedAtMs !== null" class="server-overview-info-item"><dt>{{ t('networkGeoDatabaseUpdated') }}</dt><dd>{{ formatDate(readyState.inspection.databaseUpdatedAtMs) }}</dd></div>
            <div class="server-overview-info-item"><dt>{{ t('networkSource') }}</dt><dd>{{ readyState.inspection.source === 'localAnalysis' ? t('networkLocalAnalysis') : t('networkSystemResolver') }}</dd></div>
          </dl>
        </details>
        <p v-if="readyState.inspection.databaseSource === 'dbIp'" class="server-overview-attribution"><a href="https://db-ip.com" target="_blank" rel="noopener noreferrer">IP Geolocation by DB-IP</a></p>
        <BaseButton v-if="!readyState.detailed" @click="load(true)">{{ readyState.inspection.hostKind === 'hostname' ? t('networkAnalyze') : t('networkAnalyzeDetails') }}</BaseButton>
      </template>
    </div>
  </section>
</template>
