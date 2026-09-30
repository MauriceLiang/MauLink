<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ConnectionStore } from "../stores/connections";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import ConnectionError from "../components/connection/ConnectionError.vue";
const props = defineProps<{ store: ConnectionStore; servers: ServerProfile[]; suspended?: boolean }>();
const entry = computed(() => props.store.challenge.value);
const serverId = computed(() => entry.value?.[0] ?? '');
const snapshot = computed(() => entry.value?.[1]);
const host = computed(() => snapshot.value?.hostKeyChallenge);
const auth = computed(() => snapshot.value?.authenticationChallenge);
const server = computed(() => props.servers.find(server => server.id === serverId.value));
const busy = computed(() => !!props.store.busy.value[serverId.value]);
const changed = computed(() => !!host.value?.previousFingerprintSha256);
const secret = ref('');
const advanced = ref(false);
const verified = ref(false);
const validation = ref('');
watch(() => host.value?.challengeId ?? auth.value?.challengeId, () => { secret.value = ''; advanced.value = false; verified.value = false; validation.value = ''; });
async function authenticate() {
  if (!secret.value) { validation.value = '请输入凭据后继续。'; return; }
  const value = secret.value;
  secret.value = '';
  await props.store.respondAuthentication(serverId.value, value);
}
function reject() { void props.store.respondHostKey(serverId.value, 'reject'); }
</script>
<template>
  <BaseDialog :open="!!host && !suspended" :title="changed ? '服务器身份发生变化' : '确认服务器身份'" :busy="busy" close-label="拒绝连接" panel-class="connection-dialog" @close="reject">
    <template v-if="host">
      <p class="connection-kicker" :class="{ 'connection-danger': changed }">{{ changed ? 'HOST IDENTITY CHANGED' : 'FIRST CONNECTION' }}</p>
      <p>{{ changed ? '当前指纹与已保存记录不一致。请先核实服务器变更，连接将保持暂停。' : '首次连接这台服务器，请通过独立渠道核对指纹后选择是否信任。' }}</p>
      <dl class="connection-fingerprints"><dt>主机</dt><dd>{{ host.host }}:{{ host.port }}</dd><dt>算法</dt><dd>{{ host.algorithm }}</dd><template v-if="changed"><dt>此前保存</dt><dd>{{ host.previousFingerprintSha256 }}</dd></template><dt>当前指纹</dt><dd>{{ host.fingerprintSha256 }}</dd></dl>
      <p v-if="changed" class="connection-risk" role="alert">变化可能来自服务器重装，也可能表示连接目标被替换。未核实前请拒绝连接。</p>
      <ConnectionError v-if="store.errors.value[serverId]" :error="store.errors.value[serverId]!" />
      <template v-if="changed">
        <BaseButton :aria-expanded="advanced" :disabled="busy" @click="advanced = !advanced">高级：更新信任记录</BaseButton>
        <div v-if="advanced" class="connection-risk"><label><input v-model="verified" type="checkbox" :disabled="busy" />我已通过独立渠道核实新的服务器指纹</label><BaseButton variant="danger" :disabled="busy || !verified" @click="store.respondHostKey(serverId, 'trustAndSave')">更新记录并连接</BaseButton></div>
      </template>
    </template>
    <template #footer>
      <BaseButton :disabled="busy" @click="reject">拒绝连接</BaseButton>
      <template v-if="!changed"><BaseButton :disabled="busy" @click="store.respondHostKey(serverId, 'trustOnce')">仅本次信任</BaseButton><BaseButton variant="primary" :disabled="busy" @click="store.respondHostKey(serverId, 'trustAndSave')">信任并保存</BaseButton></template>
    </template>
  </BaseDialog>
  <BaseDialog :open="!!auth && !suspended" :title="auth?.credentialKind === 'passphrase' ? '输入私钥口令' : '输入 SSH 密码'" :busy="busy" close-label="取消连接" panel-class="connection-dialog" @close="store.cancel(serverId)">
    <p>{{ server?.username }} · {{ server?.host }}</p><p class="connection-muted">凭据仅用于本次连接，不保存到资料或日志。</p>
    <form id="connection-auth-form" @submit.prevent="authenticate"><BaseInput v-model="secret" :label="auth?.credentialKind === 'passphrase' ? '私钥口令' : '密码'" type="password" autocomplete="off" :disabled="busy" :error="validation" /></form>
    <ConnectionError v-if="store.errors.value[serverId]" :error="store.errors.value[serverId]!" />
    <template #footer><BaseButton :disabled="busy" @click="store.cancel(serverId)">取消连接</BaseButton><BaseButton variant="primary" type="submit" form="connection-auth-form" :loading="busy">继续连接</BaseButton></template>
  </BaseDialog>
</template>
