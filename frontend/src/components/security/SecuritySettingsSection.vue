<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import type { CredentialRevealPolicy } from '../../../../contracts/v1/CredentialRevealPolicy';
import type { RevealMode } from '../../../../contracts/v1/RevealMode';
import BaseAlert from '../base/BaseAlert.vue';
import BaseButton from '../base/BaseButton.vue';
import BaseInput from '../base/BaseInput.vue';
import type { CredentialRevealApi } from '../../ipc/credential-reveal';
import { mapError } from '../../errors/mapper';
import { presentError } from '../../errors/presenter';
import { messages } from '../../i18n/locale';
import { securityMessages } from '../../i18n/security';

const props = defineProps<{ active: boolean; api?: CredentialRevealApi }>();
const t = messages(securityMessages);
const policy = ref<CredentialRevealPolicy | null>(null);
const selectedMode = ref<RevealMode>('deny');
const busy = ref(false);
const error = ref('');
const protectedFormOpen = ref(false);
const directFormOpen = ref(false);
const currentPassword = ref('');
const password = ref('');
const confirmPassword = ref('');
const directConfirmation = ref('');
const firstRiskConfirmed = ref(false);
const secondRiskConfirmed = ref(false);

const nativeAuthAvailable = computed(() => policy.value?.nativeAuthAvailable === true);
const hasPolicy = computed(() => !!policy.value && !!props.api);
const statusText = computed(() => policy.value ? t(`mode${capitalize(policy.value.mode)}` as keyof typeof securityMessages) : t('policyUnavailable'));
const currentPasswordRequired = computed(() => policy.value?.mode === 'protected');
const directReady = computed(() => firstRiskConfirmed.value && secondRiskConfirmed.value && directConfirmation.value === t('directConfirmationPhrase'));

function capitalize(mode: RevealMode) { return mode[0]!.toUpperCase() + mode.slice(1); }
function clearSecrets() {
  currentPassword.value = '';
  password.value = '';
  confirmPassword.value = '';
  directConfirmation.value = '';
  firstRiskConfirmed.value = false;
  secondRiskConfirmed.value = false;
}
function closeForms() {
  protectedFormOpen.value = false;
  directFormOpen.value = false;
  clearSecrets();
}
function showFailure(reason: unknown) {
  error.value = presentError(mapError(reason)).message;
  selectedMode.value = policy.value?.mode ?? 'deny';
  closeForms();
}

async function loadPolicy() {
  error.value = '';
  policy.value = null;
  selectedMode.value = 'deny';
  closeForms();
  if (!props.api) {
    error.value = t('policyUnavailable');
    return;
  }
  busy.value = true;
  try {
    const result = await props.api.getPolicy();
    policy.value = result;
    selectedMode.value = result.mode;
  } catch (reason) {
    showFailure(reason);
  } finally {
    busy.value = false;
  }
}

watch(() => props.active, active => {
  if (active) void loadPolicy();
  else closeForms();
}, { immediate: true });
onBeforeUnmount(closeForms);

async function setModeDeny() {
  if (!policy.value || !props.api || busy.value) return;
  if (policy.value.mode === 'deny') {
    selectedMode.value = 'deny';
    closeForms();
    return;
  }
  busy.value = true;
  error.value = '';
  closeForms();
  try {
    policy.value = await props.api.setDeny({ expectedRevision: policy.value.revision });
    selectedMode.value = 'deny';
  } catch (reason) {
    showFailure(reason);
  } finally {
    busy.value = false;
  }
}

function chooseMode(mode: RevealMode) {
  if (!hasPolicy.value || busy.value) return;
  if (mode === 'deny') {
    void setModeDeny();
    return;
  }
  selectedMode.value = mode;
  error.value = '';
  clearSecrets();
  protectedFormOpen.value = mode === 'protected';
  directFormOpen.value = mode === 'direct';
}

function cancelModeChange() {
  selectedMode.value = policy.value?.mode ?? 'deny';
  closeForms();
  error.value = '';
}

async function saveProtected() {
  if (!policy.value || !props.api || busy.value || password.value !== confirmPassword.value) return;
  busy.value = true;
  error.value = '';
  try {
    const next = policy.value.mode === 'protected'
      ? await props.api.changePassword({ expectedRevision: policy.value.revision, currentPassword: currentPassword.value, password: password.value, confirmPassword: confirmPassword.value })
      : await props.api.enableProtected({ expectedRevision: policy.value.revision, currentPassword: currentPasswordRequired.value ? currentPassword.value : null, password: password.value, confirmPassword: confirmPassword.value });
    policy.value = next;
    selectedMode.value = next.mode;
    closeForms();
  } catch (reason) {
    showFailure(reason);
  } finally {
    busy.value = false;
    clearSecrets();
  }
}

async function saveDirect() {
  if (!policy.value || !props.api || busy.value || !directReady.value) return;
  busy.value = true;
  error.value = '';
  try {
    const next = await props.api.enableDirect({
      expectedRevision: policy.value.revision,
      currentPassword: currentPasswordRequired.value ? currentPassword.value : null,
      confirmFirstRisk: firstRiskConfirmed.value,
      confirmSecondRisk: secondRiskConfirmed.value,
      confirmationText: directConfirmation.value,
    });
    policy.value = next;
    selectedMode.value = next.mode;
    closeForms();
  } catch (reason) {
    showFailure(reason);
  } finally {
    busy.value = false;
    clearSecrets();
  }
}

async function recoverToDeny() {
  if (!policy.value || !props.api || busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    policy.value = await props.api.recover({ expectedRevision: policy.value.revision });
    selectedMode.value = policy.value.mode;
    closeForms();
  } catch (reason) {
    showFailure(reason);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section class="settings-security" aria-labelledby="settings-security-title" :aria-busy="busy">
    <h3 id="settings-security-title">{{ t('securityPrivacy') }}</h3>
    <div class="settings-security-intro">
      <h4>{{ t('credentialRevealProtection') }}</h4>
      <p>{{ t('secretDoesNotControlConnection') }}</p>
    </div>

    <div class="settings-security-options" role="radiogroup" :aria-label="t('credentialRevealProtection')">
      <label v-for="mode in (['deny', 'protected', 'direct'] as const)" :key="mode" class="settings-security-option" :data-mode="mode" :class="{ 'is-selected': selectedMode === mode }">
        <input type="radio" name="credentialRevealMode" :value="mode" :checked="selectedMode === mode" :disabled="!hasPolicy || busy || (mode !== 'deny' && !nativeAuthAvailable)" @change="chooseMode(mode)" />
        <span class="settings-security-option-copy">
          <strong>{{ t(`mode${capitalize(mode)}` as keyof typeof securityMessages) }}</strong>
          <small>{{ t(`mode${capitalize(mode)}Description` as keyof typeof securityMessages) }}</small>
        </span>
        <span v-if="mode === 'deny'" class="settings-security-recommended">{{ t('recommended') }}</span>
      </label>
    </div>

    <p v-if="policy" class="settings-security-current" role="status">{{ t('currentMode', { mode: statusText }) }}</p>
    <BaseAlert v-if="error" role="alert">{{ error }}</BaseAlert>
    <p v-if="hasPolicy && !nativeAuthAvailable" class="settings-security-unavailable" role="status">{{ t('nativeUnavailable') }}</p>
    <p v-if="policy?.lockedUntilMs && policy.lockedUntilMs > Date.now()" class="settings-security-locked" role="status">{{ t('lockedUntil', { time: new Date(policy.lockedUntilMs).toLocaleTimeString() }) }}</p>

    <div v-if="protectedFormOpen" class="settings-security-form">
      <h4>{{ policy?.mode === 'protected' ? t('changeSecondaryPassword') : t('newSecondaryPassword') }}</h4>
      <p>{{ t('secondaryPasswordNote') }}</p>
      <BaseInput v-if="currentPasswordRequired" v-model="currentPassword" type="password" autocomplete="current-password" :label="t('currentSecondaryPassword')" :disabled="busy" />
      <BaseInput v-model="password" type="password" autocomplete="new-password" minlength="12" maxlength="128" :label="t('newSecondaryPassword')" :disabled="busy" />
      <BaseInput v-model="confirmPassword" type="password" autocomplete="new-password" minlength="12" maxlength="128" :label="t('confirmSecondaryPassword')" :disabled="busy" />
      <p v-if="password && confirmPassword && password !== confirmPassword" class="settings-security-field-error" role="alert">{{ t('passwordsDoNotMatch') }}</p>
      <div class="settings-security-actions">
        <BaseButton :disabled="busy" @click="cancelModeChange">{{ t('cancel') }}</BaseButton>
        <BaseButton variant="primary" :loading="busy" :disabled="!nativeAuthAvailable || password.length < 12 || password.length > 128 || password !== confirmPassword || (currentPasswordRequired && !currentPassword)" @click="saveProtected">{{ policy?.mode === 'protected' ? t('updateSecondaryPassword') : t('enableProtected') }}</BaseButton>
      </div>
    </div>

    <div v-if="directFormOpen" class="settings-security-form settings-security-direct-form">
      <h4>{{ t('modeDirect') }}</h4>
      <p class="settings-security-risk">{{ t('directRisk') }}</p>
      <BaseInput v-if="currentPasswordRequired" v-model="currentPassword" type="password" autocomplete="current-password" :label="t('currentSecondaryPassword')" :disabled="busy" />
      <label class="settings-security-confirm"><input v-model="firstRiskConfirmed" type="checkbox" :disabled="busy" /><span>{{ t('riskConfirmFirst') }}</span></label>
      <label class="settings-security-confirm"><input v-model="secondRiskConfirmed" type="checkbox" :disabled="busy" /><span>{{ t('riskConfirmSecond') }}</span></label>
      <BaseInput v-model="directConfirmation" autocomplete="off" :label="t('confirmationPhrase')" :placeholder="t('directConfirmationPhrase')" :disabled="busy" />
      <div class="settings-security-actions">
        <BaseButton :disabled="busy" @click="cancelModeChange">{{ t('cancel') }}</BaseButton>
        <BaseButton variant="primary" :loading="busy" :disabled="!nativeAuthAvailable || !directReady || (currentPasswordRequired && !currentPassword)" @click="saveDirect">{{ t('enableDirect') }}</BaseButton>
      </div>
    </div>

    <div v-if="policy?.mode === 'protected'" class="settings-security-recovery">
      <div><strong>{{ t('forgotSecondary') }}</strong><p>{{ t('recoverRequiresOsAuth') }}</p></div>
      <BaseButton :disabled="busy || !nativeAuthAvailable" @click="recoverToDeny">{{ t('recoverToDeny') }}</BaseButton>
    </div>
  </section>
</template>
