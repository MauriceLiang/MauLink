<script setup lang="ts">
import { computed } from 'vue';
import type { AppSettings } from '../../../../contracts/v1/AppSettings';
import BaseIcon from '../../components/base/BaseIcon.vue';
import { accentPresetColors } from '../../theme/accent';
import { messages } from '../../i18n/locale';
import { settingsMessages } from '../../i18n/settings';
import logoLight from '../../assets/app-icon-light.png';
import logoDark from '../../assets/app-icon-dark.png';

const props = defineProps<{ settings: AppSettings }>();
const t = messages(settingsMessages);
const resolvedTheme = computed(() => props.settings.theme === 'dark' || (props.settings.theme === 'system' && document.documentElement.dataset.theme === 'dark') ? 'dark' : 'light');
const previewAccent = computed(() => props.settings.accentColor === 'custom'
  ? props.settings.customAccentColor ?? '#3B82F6'
  : accentPresetColors[props.settings.accentColor]);
const logo = computed(() => props.settings.appIconStyle === 'dark' ? logoDark : logoLight);
const summary = computed(() => [
  { label: t('theme'), value: t(props.settings.theme === 'system' ? 'system' : props.settings.theme) },
  { label: t('uiDensity'), value: t(props.settings.uiDensity === 'compact' ? 'densityCompact' : 'densityStandard') },
  { label: t('sidebarWidth'), value: t(props.settings.sidebarWidth === 'narrow' ? 'sidebarNarrow' : props.settings.sidebarWidth === 'wide' ? 'sidebarWide' : 'sidebarStandard') },
  { label: t('appIconStyle'), value: t(props.settings.appIconStyle) },
]);
</script>

<template>
  <aside class="settings-appearance-preview" :aria-label="t('appearancePreviewTitle')">
    <header class="settings-appearance-preview__heading">
      <h2>{{ t('appearancePreviewTitle') }}</h2>
      <span><i aria-hidden="true"></i>{{ t('appearancePreviewSynced') }}</span>
    </header>

    <div class="settings-appearance-preview__window" :data-theme="resolvedTheme" :data-density="settings.uiDensity" :data-sidebar-width="settings.sidebarWidth" :style="{ '--preview-accent': previewAccent }">
      <div class="settings-appearance-preview__titlebar">
        <span class="settings-appearance-preview__traffic-lights" aria-hidden="true"><i></i><i></i><i></i></span>
        <div><img :src="logo" alt="" /><strong>MauLink · Production</strong></div>
      </div>
      <div class="settings-appearance-preview__app">
        <nav class="settings-appearance-preview__servers" :aria-label="t('previewServers')">
          <strong>{{ t('previewServers') }}</strong>
          <span class="is-active"><i></i>Production</span>
          <span><i></i>Development</span>
          <span><i></i>Testing</span>
        </nav>
        <div class="settings-appearance-preview__workspace">
          <div class="settings-appearance-preview__tabs"><span class="is-active">{{ t('terminal') }}</span><span>{{ t('previewFiles') }}</span></div>
          <div class="settings-appearance-preview__terminal" aria-hidden="true">
            <code><b>~</b> ssh production</code>
            <code>Connected to host</code>
            <code><b>➜</b> ls -lah</code>
          </div>
          <div class="settings-appearance-preview__files">
            <div class="settings-appearance-preview__file-heading"><strong>{{ t('previewFiles') }}</strong><strong v-if="settings.showSizeColumn">{{ t('previewSize') }}</strong></div>
            <div class="settings-appearance-preview__file-row"><span><BaseIcon name="folder" />projects</span><small v-if="settings.showSizeColumn && settings.showFolderSizes">4.8 MB</small><small v-else-if="settings.showSizeColumn">—</small></div>
            <div class="settings-appearance-preview__file-row"><span><BaseIcon name="file-text" />config.yml</span><small v-if="settings.showSizeColumn && settings.showFileSizes">2.4 KB</small></div>
            <div class="settings-appearance-preview__file-row"><span><BaseIcon name="file-text" />README.md</span><small v-if="settings.showSizeColumn && settings.showFileSizes">8.1 KB</small></div>
          </div>
        </div>
        <div class="settings-appearance-preview__toast" :data-position="settings.toastPosition">
          <BaseIcon name="success" />
          <span><strong>{{ t('toastPreviewTitle') }}</strong><small>{{ t('toastPreviewDescription') }}</small></span>
        </div>
      </div>
    </div>

    <p class="settings-appearance-preview__note">{{ t('appearancePreviewNote') }}</p>
    <section class="settings-appearance-preview__summary" :aria-label="t('appearanceCurrentSettings')">
      <h3>{{ t('appearanceCurrentSettings') }}</h3>
      <dl><div v-for="item in summary" :key="item.label"><dt>{{ item.label }}</dt><dd>{{ item.value }}</dd></div></dl>
    </section>
    <p class="settings-appearance-preview__footnote">{{ t('appearancePreviewFootnote') }}</p>
  </aside>
</template>
