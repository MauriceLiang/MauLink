<script setup lang="ts">
import { messages } from "../i18n/locale";
import { connectionMessages } from "../i18n/connection";
import { computed, ref, watch } from "vue";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ConnectionStore } from "../stores/connections";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import ConnectionError from "../components/connection/ConnectionError.vue";
const t = messages(connectionMessages);
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
  if (!secret.value) { validation.value = t('enterCredentialsToContinue'); return; }
  const value = secret.value;
  secret.value = '';
  await props.store.respondAuthentication(serverId.value, value);
}
function reject() { void props.store.respondHostKey(serverId.value, 'reject'); }
</script>
<template>
  <BaseDialog :open="!!host && !suspended" :title="changed ? t('serverIdentityChanged') : t('verifyServerIdentity')" :busy="busy" :close-label="t('rejectConnection')" panel-class="connection-dialog" @close="reject">
    <template v-if="host">
      <p class="connection-kicker" :class="{ 'connection-danger': changed }">{{ changed ? 'HOST IDENTITY CHANGED' : 'FIRST CONNECTION' }}</p>
      <p>{{ changed ? t('changedIdentityNote') : t('firstIdentityNote') }}</p>
      <dl class="connection-fingerprints"><dt>{{ t('host') }}</dt><dd>{{ host.host }}:{{ host.port }}</dd><dt>{{ t('algorithm') }}</dt><dd>{{ host.algorithm }}</dd><template v-if="changed"><dt>{{ t('previouslySaved') }}</dt><dd>{{ host.previousFingerprintSha256 }}</dd></template><dt>{{ t('currentFingerprint') }}</dt><dd>{{ host.fingerprintSha256 }}</dd></dl>
      <p v-if="changed" class="connection-risk" role="alert">{{ t('identityRisk') }}</p>
      <ConnectionError v-if="store.errors.value[serverId]" :error="store.errors.value[serverId]!" />
      <template v-if="changed">
        <BaseButton :aria-expanded="advanced" :disabled="busy" @click="advanced = !advanced">{{ t('advancedUpdateTrustedIdentity') }}</BaseButton>
        <div v-if="advanced" class="connection-risk"><label><input v-model="verified" type="checkbox" :disabled="busy" />{{ t('verifiedFingerprint') }}</label><BaseButton variant="danger" :disabled="busy || !verified" @click="store.respondHostKey(serverId, 'trustAndSave')">{{ t('updateIdentityAndConnect') }}</BaseButton></div>
      </template>
    </template>
    <template #footer>
      <BaseButton :disabled="busy" @click="reject">{{ t('rejectConnection') }}</BaseButton>
      <template v-if="!changed"><BaseButton :disabled="busy" @click="store.respondHostKey(serverId, 'trustOnce')">{{ t('trustOnce') }}</BaseButton><BaseButton variant="primary" :disabled="busy" @click="store.respondHostKey(serverId, 'trustAndSave')">{{ t('trustAndSave') }}</BaseButton></template>
    </template>
  </BaseDialog>
  <BaseDialog :open="!!auth && !suspended" :title="auth?.credentialKind === 'passphrase' ? t('enterPrivateKeyPassphrase') : t('enterSSHPassword')" :busy="busy" :close-label="t('cancelConnection')" panel-class="connection-dialog" @close="store.cancel(serverId)">
    <p>{{ server?.username }} · {{ server?.host }}</p><p class="connection-muted">{{ t('credentialNote') }}</p>
    <form id="connection-auth-form" @submit.prevent="authenticate"><BaseInput v-model="secret" :label="auth?.credentialKind === 'passphrase' ? t('passphrase') : t('password')" type="password" autocomplete="off" :disabled="busy" :error="validation" /></form>
    <ConnectionError v-if="store.errors.value[serverId]" :error="store.errors.value[serverId]!" />
    <template #footer><BaseButton :disabled="busy" @click="store.cancel(serverId)">{{ t('cancelConnection') }}</BaseButton><BaseButton variant="primary" type="submit" form="connection-auth-form" :loading="busy">{{ t('continueConnection') }}</BaseButton></template>
  </BaseDialog>
</template>
