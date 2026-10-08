<script setup lang="ts">
import { computed } from "vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import type { Language } from "../../../../contracts/v1/Language";
import { locale, messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";

const props = withDefaults(defineProps<{ server: ServerProfile; embedded?: boolean }>(), { embedded: false });
const t = messages(serverOverviewMessages);
const language = computed<Language>(() => locale.value);
function formatDate(timestamp: number) {
  return new Intl.DateTimeFormat(language.value === "en" ? "en-US" : "zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp));
}
const fields = computed(() => [
  { label: t("host"), value: props.server.host, mono: true },
  { label: t("port"), value: String(props.server.port), mono: true },
  { label: t("username"), value: props.server.username, mono: true },
  { label: t("authentication"), value: t(props.server.authType === "privateKey" ? "privateKey" : "password") },
  { label: t("privateKey"), value: t(props.server.hasPrivateKey ? "configured" : "notConfigured") },
  { label: t("savedCredential"), value: t(props.server.hasSavedCredential ? "saved" : "notSaved") },
]);
const advancedFields = computed(() => [
  { label: t("keepalive"), value: `${props.server.keepaliveIntervalSeconds} ${t("seconds")}` },
  { label: t("timeout"), value: `${props.server.connectTimeoutMs} ${t("milliseconds")}` },
  { label: t("created"), value: formatDate(props.server.createdAtMs) },
  { label: t("updated"), value: formatDate(props.server.updatedAtMs) },
]);
</script>

<template>
  <section class="server-overview-card" :class="{ 'server-overview-card--embedded': embedded }" :aria-label="t('connectionInfo')">
    <h2 v-if="!embedded">{{ t('connectionInfo') }}</h2>
    <dl class="server-overview-info-grid">
      <div v-for="field in fields" :key="field.label" class="server-overview-info-item">
        <dt>{{ field.label }}</dt>
        <dd :class="{ 'server-overview-mono': field.mono }">{{ field.value }}</dd>
      </div>
    </dl>
    <details :key="server.id" class="server-overview-extra">
      <summary>{{ t('connectionMoreDetails') }}</summary>
      <dl class="server-overview-info-grid">
        <div v-for="field in advancedFields" :key="field.label" class="server-overview-info-item"><dt>{{ field.label }}</dt><dd>{{ field.value }}</dd></div>
      </dl>
    </details>
  </section>
</template>
