<script setup lang="ts">
import { computed } from "vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { messages } from "../../i18n/locale";
import { serverOverviewMessages } from "../../i18n/server-overview";
import { connectionRoute } from "./server-details";

const props = defineProps<{ server: ServerProfile }>();
const t = messages(serverOverviewMessages);
const steps = computed(() => connectionRoute(props.server, {
  local: t("localMachine"),
  proxy: t("proxy"),
  jumpHost: t("jumpHost"),
  server: t("server"),
  socks5: t("proxySocks5"),
  httpConnect: t("proxyHttpConnect"),
}));
</script>

<template>
  <section class="server-overview-card" :aria-label="t('connectionRoute')">
    <h2>{{ t('connectionRoute') }}</h2>
    <ol class="server-overview-route">
      <li v-for="(step, index) in steps" :key="`${step.label}-${index}`" class="server-overview-route-step">
        <span v-if="index" class="server-overview-route-arrow" aria-hidden="true">→</span>
        <span class="server-overview-route-node">
          <strong>{{ step.label }}</strong>
          <span v-if="step.detail" class="server-overview-route-detail">{{ step.detail }}</span>
        </span>
      </li>
    </ol>
  </section>
</template>
