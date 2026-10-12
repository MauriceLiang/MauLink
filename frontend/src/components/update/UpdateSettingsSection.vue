<script setup lang="ts">
import { computed } from 'vue';
import BaseButton from '../base/BaseButton.vue';
import { messages } from '../../i18n/locale';
import { updaterMessages } from '../../i18n/updater';
import type { UpdaterController, UpdateErrorCode } from '../../updater/updater-types';

const props = defineProps<{ controller: UpdaterController; currentVersion: string; installBusy?: boolean }>();
const emit = defineEmits<{ 'install-request': []; 'install-deferred': [] }>();
const t = messages(updaterMessages);
const state = computed(() => props.controller.state.value);
const percentage = computed(() => {
  const { downloadedBytes, totalBytes } = state.value;
  return totalBytes ? Math.min(100, Math.floor(downloadedBytes / totalBytes * 100)) : null;
});
const downloadedLabel = computed(() => t('downloadedBytes', { bytes: formatBytes(state.value.downloadedBytes) }));

function formatBytes(value: number) {
  if (value < 1024 * 1024) return `${Math.max(0, Math.round(value / 1024))} KiB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
}
function statusText() {
  switch (state.value.phase) {
    case 'checking': return t('checking');
    case 'upToDate': return t('upToDate');
    case 'available': return t('foundUpdate', { version: state.value.latestVersion ?? '—' });
    case 'downloading': return t('downloading');
    case 'downloaded': return t('downloaded');
    case 'confirmingInstall': return t('confirmingInstall');
    case 'installing': return t('installing');
    case 'unsupported': return t(props.controller.adapterKind === 'native' ? 'notConfigured' : 'onlyDesktop');
    case 'error': return t(errorMessage(state.value.errorCode));
    default: return '';
  }
}
function errorMessage(code?: UpdateErrorCode): keyof typeof updaterMessages {
  switch (code) {
    case 'network': return 'errorNetwork';
    case 'metadata': return 'errorMetadata';
    case 'signature': return 'errorSignature';
    case 'storage': return 'errorStorage';
    case 'permission': return 'errorPermission';
    case 'install': return 'errorInstall';
    case 'unsupported': return props.controller.adapterKind === 'native' ? 'notConfigured' : 'onlyDesktop';
    default: return 'errorUnknown';
  }
}
function check() { void props.controller.checkForUpdates(); }
function dismiss() { void props.controller.dismiss(); }
function download() { void props.controller.downloadUpdate(); }
function requestInstall() {
  props.controller.requestInstall();
  emit('install-request');
}
function deferInstall() { emit('install-deferred'); }
</script>

<template>
  <section id="settings-updates" class="update-settings" tabindex="-1" aria-labelledby="update-settings-title">
    <div class="update-settings__heading">
      <div>
        <h2 id="update-settings-title">{{ t('title') }}</h2>
        <p>{{ t('description') }}</p>
      </div>
      <div class="update-settings__version"><span>{{ t('currentVersion') }}</span><code>{{ currentVersion }}</code></div>
    </div>

    <p v-if="statusText()" class="update-settings__status" role="status" aria-live="polite">{{ statusText() }}</p>
    <div v-if="state.phase === 'available' && state.releaseNotes" class="update-settings__notes">
      <h3>{{ t('releaseNotes') }}</h3>
      <p>{{ state.releaseNotes }}</p>
    </div>
    <div v-if="state.phase === 'downloading'" class="update-settings__progress">
      <div
        class="update-settings__progress-track"
        role="progressbar"
        :aria-label="t('downloading')"
        aria-valuemin="0"
        :aria-valuemax="percentage === null ? undefined : 100"
        :aria-valuenow="percentage ?? undefined"
        :aria-valuetext="downloadedLabel"
      >
        <div class="update-settings__progress-fill" :class="{ 'is-indeterminate': percentage === null }" :style="percentage === null ? undefined : { width: `${percentage}%` }" />
      </div>
      <span>{{ percentage === null ? downloadedLabel : `${downloadedLabel} · ${percentage}%` }}</span>
    </div>

    <div class="update-settings__actions">
      <BaseButton v-if="['idle', 'upToDate', 'error'].includes(state.phase)" variant="secondary" :disabled="installBusy" @click="check">{{ t('check') }}</BaseButton>
      <template v-else-if="state.phase === 'available'">
        <BaseButton variant="secondary" :disabled="installBusy" @click="dismiss">{{ t('later') }}</BaseButton>
        <BaseButton variant="primary" :disabled="installBusy" @click="download">{{ t('download') }}</BaseButton>
      </template>
      <template v-else-if="state.phase === 'downloaded'">
        <BaseButton variant="secondary" :disabled="installBusy" @click="deferInstall">{{ t('installLater') }}</BaseButton>
        <BaseButton variant="primary" :disabled="installBusy" @click="requestInstall">{{ t('installAndRestart') }}</BaseButton>
      </template>
      <BaseButton v-else-if="state.phase === 'confirmingInstall'" variant="secondary" :disabled="installBusy" @click="props.controller.cancelInstallRequest()">{{ t('continueWorking') }}</BaseButton>
    </div>
  </section>
</template>
