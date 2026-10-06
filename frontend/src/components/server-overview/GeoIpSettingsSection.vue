<script setup lang="ts">
import { onMounted, ref } from 'vue';
import type { GeoIpDatabaseStatus } from '../../../../contracts/v1/GeoIpDatabaseStatus';
import type { GeoIpApi } from '../../ipc/geoip';
import { locale, messages } from '../../i18n/locale';
import { geoIpMessages } from '../../i18n/geoip';
import { mapError } from '../../errors/mapper';
import { presentError } from '../../errors/presenter';
import BaseButton from '../base/BaseButton.vue';
import BaseSelect from '../base/BaseSelect.vue';
import BaseAlert from '../base/BaseAlert.vue';
const props = defineProps<{ api: GeoIpApi }>();
const emit = defineEmits<{ changed: []; busy: [value: boolean] }>();
const t = messages(geoIpMessages);
const status = ref<GeoIpDatabaseStatus | null>(null);
const busy = ref(false);
const loading = ref(true);
const error = ref('');
const completed = ref('');
function date(timestamp: number) { return new Intl.DateTimeFormat(locale.value === 'en' ? 'en-US' : 'zh-CN', { dateStyle: 'medium' }).format(new Date(timestamp)); }
async function load() {
  loading.value = true;
  try { status.value = await props.api.get(); }
  catch (reason) { error.value = presentError(mapError(reason)).message; }
  finally { loading.value = false; }
}
async function perform(action: () => Promise<GeoIpDatabaseStatus | null>, message: string, changesDatabase = true) {
  if (busy.value) return;
  busy.value = true; emit('busy', true); error.value = ''; completed.value = '';
  try {
    const next = await action();
    if (next) { status.value = next; completed.value = message; if (changesDatabase) emit('changed'); }
  } catch (reason) { error.value = presentError(mapError(reason)).message; await load(); if (changesDatabase) emit('changed'); }
  finally { busy.value = false; emit('busy', false); }
}
function configure(value: unknown) { if (typeof value === 'string') void perform(() => props.api.configure(Number(value)), t('frequencySaved'), false); }
onMounted(load);
</script>
<template>
  <section class="geoip-settings" :aria-label="t('title')" :aria-busy="busy || loading">
    <h3>{{ t('title') }}</h3>
    <p class="geoip-intro">{{ t('intro') }}</p>
    <div class="geoip-status">
      <template v-if="status">
        <div v-for="slot in ['location', 'asn'] as const" :key="slot" class="geoip-database-row">
          <div><strong>{{ t(slot) }}</strong><span v-if="status[slot]">{{ status[slot]!.fileName }}</span><span v-else>{{ t('notConfigured') }}</span></div>
          <span v-if="status[slot]" class="geoip-database-date">{{ status[slot]!.available ? date(status[slot]!.buildAtMs) : t('unavailable') }}</span>
        </div>
      </template>
      <p v-else-if="loading">{{ t('loading') }}</p>
      <BaseButton v-else :disabled="busy || loading" @click="error = ''; load()">{{ t('retry') }}</BaseButton>
    </div>
    <div class="geoip-actions">
      <BaseButton variant="primary" :loading="busy" :disabled="loading" @click="perform(api.update, t('updated'))">{{ status?.automaticUpdates ? t('updateNow') : t('install') }}</BaseButton>
      <BaseButton :disabled="busy || loading" @click="perform(api.import, t('imported'))">{{ t('import') }}</BaseButton>
      <BaseButton variant="danger" :disabled="busy || !(status?.location || status?.asn)" @click="perform(api.delete, t('deleted'))">{{ t('delete') }}</BaseButton>
    </div>
    <BaseSelect :model-value="String(status?.updateIntervalDays ?? 30)" :disabled="busy || !status" :label="t('frequency')" :options="[{value:'0',label:t('manual')},{value:'1',label:t('daily')},{value:'7',label:t('weekly')},{value:'30',label:t('monthly')}]" @update:model-value="configure" />
    <p class="geoip-note">{{ t(status?.automaticUpdates ? 'automaticNote' : 'localNote') }}</p>
    <p v-if="status?.lastCheckedAtMs" class="geoip-note">{{ t('lastChecked') }}：{{ date(status.lastCheckedAtMs) }}</p>
    <BaseAlert v-if="error">{{ error }}</BaseAlert>
    <BaseAlert v-else-if="status?.lastError">{{ presentError(mapError({ messageKey: status.lastError, code: 'INTERNAL', params: {}, retryable: true, action: 'retry', stage: null, requestId: null, details: null })).message }}</BaseAlert>
    <p v-if="completed" class="geoip-completed" role="status">{{ completed }}</p>
    <p class="geoip-note">{{ t('immediate') }}</p>
    <p class="geoip-attribution"><a href="https://db-ip.com/db/lite.php" target="_blank" rel="noopener noreferrer">IP Geolocation by DB-IP · CC BY 4.0</a></p>
  </section>
</template>
