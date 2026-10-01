<script setup lang="ts">
import { messages } from "../../i18n/locale";
import { connectionMessages } from "../../i18n/connection";
import { computed } from "vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { connectionStateText } from "../../i18n/connections";
import type { ConnectionStore } from "../../stores/connections";
import { isFinished } from "../../stores/connections";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import ConnectionError from "./ConnectionError.vue";
const t = messages(connectionMessages);
const props = defineProps<{ server: ServerProfile; store: ConnectionStore; readOnly: boolean }>();
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const busy = computed(() => !!props.store.busy.value[props.server.id]);
const transportError = computed(() => props.store.errors.value[props.server.id]);
const error = computed(() => transportError.value ?? snapshot.value?.error);
const active = computed(() => snapshot.value && !isFinished(snapshot.value));
const retryable = computed(() => error.value && presentError(error.value).retryable && !active.value);

</script>
<template>
  <section class="connection-panel" :aria-label="t('connection', {name: server.name})" :aria-busy="busy">
    <h1>{{ server.name }}</h1><p class="connection-muted">{{ server.username }}@{{ server.host }}:{{ server.port }} · {{ snapshot ? connectionStateText(snapshot.state) : t('notConnected') }}</p>
    <p role="status">{{ busy ? t('working') : snapshot ? connectionStateText(snapshot.state) : t('connectToViewSSHStatus') }}</p>
    <p v-if="snapshot?.state === 'ready'" class="connection-muted">{{ t('sshAuthenticationCompleted') }}</p>
    <p v-if="server.jumpHost || server.proxyHost" class="connection-muted">{{ t('route') }}{{ server.jumpHost ? t('jumpRoute', {host: server.jumpHost, port: server.jumpPort}) : t('directConnection') }}{{ server.proxyHost ? t('proxyRoute', {type: server.proxyType ?? '—', host: server.proxyHost, port: server.proxyPort ?? '—'}) : '' }}</p>
    <ConnectionError v-if="error" :error="error" />
    <div class="connection-actions">
      <BaseButton v-if="!active && (!error || retryable)" variant="primary" :disabled="readOnly || !!store.challenge.value" :loading="busy" @click="store.start(server)">{{ retryable ? t('retryConnection') : t('connect') }}</BaseButton>
      <BaseButton v-if="active && snapshot?.state !== 'ready'" :disabled="busy" @click="store.cancel(server.id)">{{ t('cancelConnection') }}</BaseButton>
      <BaseButton v-if="snapshot?.state === 'ready'" :disabled="busy" @click="store.disconnect(server.id)">{{ t('disconnect') }}</BaseButton>
      <BaseButton v-if="transportError && active" :disabled="busy" @click="store.refresh(server.id)">{{ t('refreshConnectionStatus') }}</BaseButton>
      <BaseButton v-if="error && !active && !retryable" :disabled="busy" @click="store.dismiss(server.id)">{{ t('dismissError') }}</BaseButton>
      <slot />
    </div>
  </section>
</template>
