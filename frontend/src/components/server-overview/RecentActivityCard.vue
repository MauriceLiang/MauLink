<script setup lang="ts">
import { computed } from "vue";
import type { ConnectionSnapshot } from "../../../../contracts/v1/ConnectionSnapshot";
import type { ServerRuntimeStats } from "../../../../contracts/v1/ServerRuntimeStats";
import { locale, messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";

const props = withDefaults(defineProps<{ stats: ServerRuntimeStats | null; snapshot?: ConnectionSnapshot; state: "loading" | "ready" | "error"; embedded?: boolean }>(), { embedded: false });
const t = messages(serverOverviewMessages);
const currentWorkspace = computed(() => props.snapshot?.mode === "workspace" ? props.snapshot : null);
const lastSuccessAtMs = computed(() => {
  const current = currentWorkspace.value;
  return current?.state === "ready" && current.updatedAtMs > (props.stats?.lastSuccessAtMs ?? 0)
    ? current.updatedAtMs
    : props.stats?.lastSuccessAtMs ?? null;
});
const lastFailureAtMs = computed(() => {
  const current = currentWorkspace.value;
  return current?.state === "failed" && current.updatedAtMs > (props.stats?.lastFailureAtMs ?? 0)
    ? current.updatedAtMs
    : props.stats?.lastFailureAtMs ?? null;
});
const lastFailureCode = computed(() => {
  const current = currentWorkspace.value;
  return current?.state === "failed" && current.updatedAtMs === lastFailureAtMs.value
    ? current.error?.code ?? null
    : props.stats?.lastFailureCode ?? null;
});
const hasActivity = computed(() => lastSuccessAtMs.value !== null || lastFailureAtMs.value !== null || props.stats?.lastPreflightAtMs != null);

function formatDate(timestamp: number) {
  return new Intl.DateTimeFormat(locale.value === "en" ? "en-US" : "zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}
</script>

<template>
  <section class="server-overview-card server-overview-activity" :class="{ 'server-overview-card--embedded': embedded }" :aria-label="t('recentActivity')">
    <h2 v-if="!embedded">{{ t('recentActivity') }}</h2>
    <p v-if="state === 'loading' && !hasActivity" class="server-overview-activity-empty" role="status">{{ t('activityLoading') }}</p>
    <p v-else-if="state === 'error' && !hasActivity" class="server-overview-activity-empty" role="alert">{{ t('activityUnavailable') }}</p>
    <p v-else-if="!hasActivity" class="server-overview-activity-empty">{{ t('noRecentActivity') }}</p>
    <dl v-else class="server-overview-info-grid">
      <div class="server-overview-info-item">
        <dt>{{ t('lastSuccessfulConnection') }}</dt>
        <dd>{{ lastSuccessAtMs === null ? '—' : formatDate(lastSuccessAtMs) }}</dd>
      </div>
      <div v-if="lastFailureAtMs !== null" class="server-overview-info-item">
        <dt>{{ t('lastFailedConnection') }}</dt>
        <dd>{{ formatDate(lastFailureAtMs) }}<code v-if="lastFailureCode"> · {{ lastFailureCode }}</code></dd>
      </div>
      <div v-if="stats?.lastPreflightAtMs != null" class="server-overview-info-item">
        <dt>{{ t('lastPreflight') }}</dt>
        <dd>
          <template v-if="stats.lastPreflightLatencyMs !== null">{{ stats.lastPreflightLatencyMs }} {{ t('milliseconds') }} · </template>
          {{ formatDate(stats.lastPreflightAtMs) }}
        </dd>
      </div>
    </dl>
  </section>
</template>
