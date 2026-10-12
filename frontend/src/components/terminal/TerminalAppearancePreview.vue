<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import type { AppSettings } from '../../../../contracts/v1/AppSettings';
import { messages } from '../../i18n/locale';
import { terminalMessages } from '../../i18n/terminal';
import { terminalBackgroundImageStyle, terminalBackgroundOverlayStyle } from '../../terminal/background';
import { resolveTerminalTheme } from '../../terminal/theme';

const props = withDefaults(defineProps<{
  settings: AppSettings;
  backgroundUrl?: string;
  loading?: boolean;
  emptyImage?: boolean;
}>(), { backgroundUrl: '', loading: false, emptyImage: false });
const t = messages(terminalMessages);
const systemIsDark = ref(false);
const previewTheme = computed(() => resolveTerminalTheme(props.settings, props.settings.theme, systemIsDark.value));
const imageStyle = computed(() => terminalBackgroundImageStyle(props.backgroundUrl, props.settings.terminalBackgroundImage));
const overlayStyle = computed(() => terminalBackgroundOverlayStyle(props.settings.terminalBackgroundImage));

let colorScheme: MediaQueryList | null = null;
function updateColorScheme(event: MediaQueryList | MediaQueryListEvent) { systemIsDark.value = event.matches; }
onMounted(() => {
  if (typeof window.matchMedia !== 'function') return;
  colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
  updateColorScheme(colorScheme);
  colorScheme.addEventListener('change', updateColorScheme);
});
onBeforeUnmount(() => colorScheme?.removeEventListener('change', updateColorScheme));
</script>

<template>
  <div
    class="terminal-appearance-preview"
    role="img"
    :aria-label="t('terminalPreview')"
    :aria-busy="loading"
    :style="{
      backgroundColor: previewTheme.theme.background,
      color: previewTheme.theme.foreground,
      fontFamily: settings.terminalFontFamily,
      fontSize: `${settings.terminalFontSize}px`,
      lineHeight: settings.terminalLineHeight,
    }"
  >
    <div v-if="settings.terminalThemeMode === 'image' && backgroundUrl" class="terminal-appearance-preview-image" :style="imageStyle" />
    <div v-if="settings.terminalThemeMode === 'image' && backgroundUrl" class="terminal-appearance-preview-overlay" :style="overlayStyle" />
    <div class="terminal-appearance-preview-content">
      <div><span :style="{ color: previewTheme.theme.cursor }">maurice@server:~$</span> ls -la</div>
      <div>Documents&nbsp; Downloads&nbsp; project</div>
      <div>
        <span :style="{ color: previewTheme.theme.cursor }">maurice@server:~$</span>
        <span class="terminal-appearance-preview-selection" :style="{ backgroundColor: previewTheme.theme.selectionBackground }"> cat README.md</span>
        <span :style="{ color: previewTheme.theme.cursor }"> █</span>
      </div>
    </div>
    <span v-if="emptyImage && !backgroundUrl" class="terminal-appearance-preview-empty">{{ t(loading ? 'appearancePreviewLoading' : 'imageChoosePrompt') }}</span>
  </div>
</template>
