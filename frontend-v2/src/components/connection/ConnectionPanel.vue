<script setup lang="ts">
import { computed } from "vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import { connectionStateText } from "../../i18n/connections";
import type { ConnectionStore } from "../../stores/connections";
import { isFinished } from "../../stores/connections";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import ConnectionError from "./ConnectionError.vue";
const props = defineProps<{ server: ServerProfile; store: ConnectionStore; readOnly: boolean }>();
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const busy = computed(() => !!props.store.busy.value[props.server.id]);
const transportError = computed(() => props.store.errors.value[props.server.id]);
const error = computed(() => transportError.value ?? snapshot.value?.error);
const active = computed(() => snapshot.value && !isFinished(snapshot.value));
const retryable = computed(() => error.value && presentError(error.value).retryable && !active.value);

</script>
<template>
  <section class="connection-panel" :aria-label="`${server.name} 连接`" :aria-busy="busy">
    <h1>{{ server.name }}</h1><p class="connection-muted">{{ server.username }}@{{ server.host }}:{{ server.port }} · {{ snapshot ? connectionStateText(snapshot.state) : '尚未连接' }}</p>
    <p role="status">{{ busy ? '正在处理…' : snapshot ? connectionStateText(snapshot.state) : '连接后将显示 SSH 状态。' }}</p>
    <p v-if="snapshot?.state === 'ready'" class="connection-muted">SSH 认证已完成。终端工作区将在下一阶段接入。</p>
    <p v-if="server.jumpHost || server.proxyHost" class="connection-muted">连接路径：{{ server.jumpHost ? `跳板机 ${server.jumpHost}:${server.jumpPort}` : '直接连接' }}{{ server.proxyHost ? ` · ${server.proxyType} 代理 ${server.proxyHost}:${server.proxyPort}` : '' }}</p>
    <ConnectionError v-if="error" :error="error" />
    <div class="connection-actions">
      <BaseButton v-if="!active && (!error || retryable)" variant="primary" :disabled="readOnly || !!store.challenge.value" :loading="busy" @click="store.start(server)">{{ retryable ? '重试连接' : '连接' }}</BaseButton>
      <BaseButton v-if="active && snapshot?.state !== 'ready'" :disabled="busy" @click="store.cancel(server.id)">取消连接</BaseButton>
      <BaseButton v-if="snapshot?.state === 'ready'" :disabled="busy" @click="store.disconnect(server.id)">断开连接</BaseButton>
      <BaseButton v-if="transportError && active" :disabled="busy" @click="store.refresh(server.id)">刷新连接状态</BaseButton>
      <BaseButton v-if="error && !active && !retryable" :disabled="busy" @click="store.dismiss(server.id)">关闭错误</BaseButton>
      <slot />
    </div>
  </section>
</template>
