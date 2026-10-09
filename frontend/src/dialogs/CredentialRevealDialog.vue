<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue';
import type { CredentialRevealPolicy } from '../../../contracts/v1/CredentialRevealPolicy';
import type { CredentialKind } from '../../../contracts/v1/CredentialKind';
import BaseAlert from '../components/base/BaseAlert.vue';
import BaseButton from '../components/base/BaseButton.vue';
import BaseDialog from '../components/base/BaseDialog.vue';
import BaseInput from '../components/base/BaseInput.vue';
import type { CredentialRevealApi } from '../ipc/credential-reveal';
import { mapError } from '../errors/mapper';
import { presentError } from '../errors/presenter';
import { messages } from '../i18n/locale';
import { securityMessages } from '../i18n/security';

type RevealServer = { id: string; name: string; authType: 'password' | 'privateKey'; hasSavedCredential: boolean };
const props = defineProps<{ open: boolean; server: RevealServer | null; api: CredentialRevealApi }>();
const emit = defineEmits<{ close: []; openSecurity: [] }>();
const t = messages(securityMessages);
const policy = ref<CredentialRevealPolicy | null>(null);
const secondaryPassword = ref('');
const revealedValue = ref<string | null>(null);
const revealedKind = ref<CredentialKind | null>(null);
const error = ref('');
const busy = ref(false);
let requestGeneration = 0;
let hideTimer: ReturnType<typeof setTimeout> | undefined;

function clearTimer() {
  if (hideTimer !== undefined) clearTimeout(hideTimer);
  hideTimer = undefined;
}
function hideValue() {
  requestGeneration++;
  clearTimer();
  secondaryPassword.value = '';
  revealedValue.value = null;
  revealedKind.value = null;
  error.value = '';
  busy.value = false;
}
function clearSession() {
  hideValue();
  policy.value = null;
}
function onBlur() { hideValue(); }
function onVisibilityChange() { if (document.visibilityState !== 'visible') hideValue(); }

async function loadPolicy(serverId: string, generation: number) {
  busy.value = true;
  try {
    const result = await props.api.getPolicy();
    if (generation !== requestGeneration || !props.open || props.server?.id !== serverId) return;
    policy.value = result;
  } catch (reason) {
    if (generation !== requestGeneration || !props.open || props.server?.id !== serverId) return;
    error.value = presentError(mapError(reason)).message || t('revealUnavailable');
  } finally {
    if (generation === requestGeneration) busy.value = false;
  }
}

watch([() => props.open, () => props.server?.id], ([open, serverId]) => {
  clearSession();
  if (open && serverId) {
    const generation = requestGeneration;
    void loadPolicy(serverId, generation);
  }
}, { immediate: true });

watch(() => props.open, open => {
  if (open) {
    window.addEventListener('blur', onBlur);
    document.addEventListener('visibilitychange', onVisibilityChange);
  } else {
    window.removeEventListener('blur', onBlur);
    document.removeEventListener('visibilitychange', onVisibilityChange);
  }
}, { immediate: true });

onBeforeUnmount(() => {
  clearSession();
  window.removeEventListener('blur', onBlur);
  document.removeEventListener('visibilitychange', onVisibilityChange);
});

async function reveal() {
  const server = props.server;
  const currentPolicy = policy.value;
  if (!server || !server.hasSavedCredential || !currentPolicy || busy.value) return;
  if (currentPolicy.mode === 'deny') return;
  if (currentPolicy.mode === 'protected' && !secondaryPassword.value) return;
  clearTimer();
  revealedValue.value = null;
  revealedKind.value = null;
  error.value = '';
  busy.value = true;
  const generation = ++requestGeneration;
  const serverId = server.id;
  const suppliedPassword = secondaryPassword.value;
  secondaryPassword.value = '';
  try {
    const result = await props.api.reveal({ serverId, secondaryPassword: currentPolicy.mode === 'protected' ? suppliedPassword : null });
    if (generation !== requestGeneration || !props.open || props.server?.id !== serverId) return;
    revealedValue.value = result.value;
    revealedKind.value = result.kind;
    hideTimer = setTimeout(hideValue, 15_000);
  } catch (reason) {
    if (generation !== requestGeneration || !props.open || props.server?.id !== serverId) return;
    error.value = presentError(mapError(reason)).message;
  } finally {
    if (generation === requestGeneration) busy.value = false;
  }
}
</script>

<template>
  <BaseDialog :open="open && !!server" :title="server?.name ?? t('revealSavedPassword')" :close-label="t('close')" panel-class="credential-reveal-dialog" @close="emit('close')">
    <BaseAlert v-if="error" role="alert">{{ error }}</BaseAlert>
    <p v-if="policy?.mode === 'deny'" class="credential-reveal-denied" role="status">{{ t('revealDenied') }}</p>
    <template v-else-if="policy?.mode === 'protected' && !revealedValue">
      <p>{{ t('protectedRevealPrompt') }}</p>
      <BaseInput v-model="secondaryPassword" type="password" autocomplete="current-password" :label="t('currentSecondaryPassword')" :disabled="busy" @keydown.enter.prevent="reveal" />
    </template>
    <p v-else-if="policy?.mode === 'direct' && !revealedValue">{{ t('directRevealPrompt') }}</p>
    <section v-if="revealedValue && revealedKind" class="credential-reveal-value" aria-live="polite">
      <label>{{ t(revealedKind === 'password' ? 'revealSavedPassword' : 'revealSavedPassphrase') }}</label>
      <code>{{ revealedValue }}</code>
      <p>{{ t('revealAfterHide') }}</p>
    </section>
    <p v-if="!policy && !busy && !error" role="status">{{ t('revealUnavailable') }}</p>
    <template #footer>
      <BaseButton v-if="policy?.mode === 'deny'" @click="emit('openSecurity')">{{ t('openSecuritySettings') }}</BaseButton>
      <BaseButton v-else-if="revealedValue" @click="hideValue">{{ t('hideCredential') }}</BaseButton>
      <BaseButton v-else-if="policy?.mode === 'protected'" variant="primary" :loading="busy" :disabled="busy || !secondaryPassword" @click="reveal">{{ t('reveal') }}</BaseButton>
      <BaseButton v-else-if="policy?.mode === 'direct'" variant="primary" :loading="busy" :disabled="busy" @click="reveal">{{ t('showSavedCredential') }}</BaseButton>
      <BaseButton @click="emit('close')">{{ t('close') }}</BaseButton>
    </template>
  </BaseDialog>
</template>
