<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import type { HostKeyRecord } from "../../../../contracts/v1/HostKeyRecord";
import { locale, messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";
import type { createHostKeysApi } from "../../ipc/host-keys";
import BaseButton from "../base/BaseButton.vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";

const props = defineProps<{ server: ServerProfile; api: ReturnType<typeof createHostKeysApi> }>();
type LoadState = { status: "loading" | "empty" | "error" } | { status: "available"; record: HostKeyRecord };
const t = messages(serverOverviewMessages);
const state = ref<LoadState>({ status: "loading" });
let requestVersion = 0;

async function load() {
  const version = ++requestVersion;
  state.value = { status: "loading" };
  try {
    const record = await props.api.get({ host: props.server.host, port: props.server.port });
    if (version === requestVersion) state.value = record ? { status: "available", record } : { status: "empty" };
  } catch {
    if (version === requestVersion) state.value = { status: "error" };
  }
}

function formatDate(timestamp: number) {
  return new Intl.DateTimeFormat(locale.value === "en" ? "en-US" : "zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp));
}

watch(() => [props.server.host, props.server.port] as const, () => { void load(); }, { immediate: true });
onBeforeUnmount(() => { requestVersion += 1; });
</script>

<template>
  <section class="server-overview-card server-overview-host-identity" :aria-label="t('securityIdentity')">
    <h2>{{ t('securityIdentity') }}</h2>
    <p class="server-overview-host-identity-note">{{ t('trustRecordNotice') }}</p>
    <div class="server-overview-trust-state" :aria-live="state.status === 'loading' ? 'polite' : 'off'">
      <p v-if="state.status === 'loading'">{{ t('loadingTrustRecord') }}</p>
      <template v-else-if="state.status === 'available'">
        <strong>{{ t('trustRecordAvailable') }}</strong>
        <dl class="server-overview-info-grid">
          <div class="server-overview-info-item"><dt>{{ t('keyAlgorithm') }}</dt><dd class="server-overview-mono">{{ state.record.algorithm }}</dd></div>
          <div class="server-overview-info-item"><dt>{{ t('fingerprint') }}</dt><dd class="server-overview-mono server-overview-fingerprint">{{ state.record.fingerprintSha256 }}</dd></div>
          <div class="server-overview-info-item"><dt>{{ t('trustedAt') }}</dt><dd>{{ formatDate(state.record.trustedAtMs) }}</dd></div>
        </dl>
      </template>
      <p v-else-if="state.status === 'empty'">{{ t('noTrustRecord') }}</p>
      <div v-else class="server-overview-trust-error" role="alert">
        <p>{{ t('trustRecordUnavailable') }}</p>
        <BaseButton @click="load">{{ t('retry') }}</BaseButton>
      </div>
    </div>
  </section>
</template>
