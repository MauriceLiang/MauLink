<script setup lang="ts">
import { computed } from 'vue';
import { messages } from '../../i18n/locale';
import { settingsMessages } from '../../i18n/settings';
import BaseButton from '../../components/base/BaseButton.vue';
import BaseIcon from '../../components/base/BaseIcon.vue';
import type { SettingsSection } from './settings-navigation';
import { settingsNavigation } from './settings-navigation';

const props = defineProps<{ section: SettingsSection; hasGeoIp: boolean; busy?: boolean }>();
const emit = defineEmits<{ 'update:section': [section: SettingsSection]; back: [] }>();
const t = messages(settingsMessages);
const items = computed(() => settingsNavigation(props.hasGeoIp));
</script>

<template>
  <aside class="settings-sidebar" :aria-label="t('sections')">
    <div class="settings-sidebar__top"><BaseButton variant="ghost" size="md" block @click="emit('back')"><BaseIcon name="arrow-left" />{{ t('backToWorkspace') }}</BaseButton></div>
    <h2 class="settings-sidebar__title">{{ t('settings') }}</h2>
    <nav class="settings-sidebar__navigation" :aria-label="t('sections')">
      <BaseButton v-for="item in items" :key="item.id" variant="ghost" size="md" block class="settings-sidebar__item" :aria-label="t(item.label)" :aria-current="section === item.id ? 'page' : undefined" :disabled="busy" @click="emit('update:section', item.id)">
        <BaseIcon :name="item.icon" /><span>{{ t(item.label) }}</span>
      </BaseButton>
    </nav>
  </aside>
</template>
