import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { createIpcClient } from '../src/ipc/client';
import { createSettingsApi } from '../src/ipc/settings';
import { createCredentialRevealApi } from '../src/ipc/credential-reveal';
import { createTerminalPreferences } from '../src/terminal/preferences';
import { createSettingsMock } from '../src/harness/settings-fixtures';
import { createCredentialRevealMock } from '../src/harness/credential-reveal-fixtures';
import { locale } from '../src/i18n/locale';
import { securityMessages } from '../src/i18n/security';
import SettingsDialog from '../src/dialogs/SettingsDialog.vue';
import type { BackgroundImagesApi } from '../src/ipc/background-images';

const wrappers: VueWrapper[] = [];
const backgroundImages = { select: vi.fn(async () => null), get: vi.fn(), resolve: vi.fn(), delete: vi.fn(async () => {}) } as unknown as BackgroundImagesApi;
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(element => element.textContent?.trim() === label)!;
function mountSettings(nativeAuthAvailable = true) {
  const settings = createSettingsMock();
  const security = createCredentialRevealMock({ nativeAuthAvailable });
  const preferences = createTerminalPreferences(createSettingsApi(createIpcClient(settings.transport)), () => {});
  const preferencesSave = vi.spyOn(preferences, 'save');
  const wrapper = mount(SettingsDialog, {
    attachTo: document.body,
    props: { open: false, preferences, backgroundImages, credentialReveal: createCredentialRevealApi(createIpcClient(security.transport)), initialSection: 'security' },
  });
  void wrapper.setProps({ open: true });
  wrappers.push(wrapper);
  return { wrapper, security, preferencesSave };
}

afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ''; locale.value = 'zh-CN'; vi.restoreAllMocks(); });

describe('Security & Privacy settings', () => {
  it('shows the policy entry and keeps policy changes out of ordinary AppSettings saves', async () => {
    const { security, preferencesSave } = mountSettings();
    await flushPromises();
    expect(button('安全与隐私')).toBeDefined();
    expect(document.querySelector('[role="status"]')?.textContent).toContain('禁止查看明文');
    expect(document.querySelector('form')).toBeNull();

    document.querySelector<HTMLInputElement>('input[name="credentialRevealMode"][value="protected"]')!.click();
    await flushPromises();
    const passwordFields = [...document.querySelectorAll<HTMLInputElement>('.settings-security-form input[type="password"]')];
    passwordFields[0]!.value = 'fixture-secondary-password';
    passwordFields[0]!.dispatchEvent(new Event('input', { bubbles: true }));
    passwordFields[1]!.value = 'fixture-secondary-password';
    passwordFields[1]!.dispatchEvent(new Event('input', { bubbles: true }));
    await flushPromises();
    button('启用二级密码保护').click();
    await flushPromises();

    expect(security.current()).toMatchObject({ mode: 'protected', hasSecondaryPassword: true });
    expect(preferencesSave).not.toHaveBeenCalled();
  });

  it('requires two risk acknowledgements and the exact phrase before enabling direct view', async () => {
    const { security } = mountSettings();
    await flushPromises();
    document.querySelector<HTMLInputElement>('input[name="credentialRevealMode"][value="direct"]')!.click();
    await flushPromises();
    const enable = button('完成系统验证并启用');
    expect(enable.disabled).toBe(true);
    const checkboxes = [...document.querySelectorAll<HTMLInputElement>('.settings-security-direct-form input[type="checkbox"]')];
    checkboxes[0]!.click();
    checkboxes[1]!.click();
    const phrase = document.querySelector<HTMLInputElement>('.settings-security-direct-form input:not([type="checkbox"])')!;
    phrase.value = securityMessages.directConfirmationPhrase[0];
    phrase.dispatchEvent(new Event('input', { bubbles: true }));
    await flushPromises();
    expect(enable.disabled).toBe(false);
    enable.click();
    await flushPromises();
    expect(security.current().mode).toBe('direct');

    document.querySelector<HTMLInputElement>('input[name="credentialRevealMode"][value="deny"]')!.click();
    await flushPromises();
    expect(security.current().mode).toBe('deny');
  });

  it('keeps enabling options disabled when native authentication is unavailable', async () => {
    mountSettings(false);
    await flushPromises();
    expect(document.querySelector('.settings-security-unavailable')?.textContent).toContain('系统身份验证');
    expect(document.querySelector<HTMLInputElement>('input[name="credentialRevealMode"][value="protected"]')!.disabled).toBe(true);
    expect(document.querySelector<HTMLInputElement>('input[name="credentialRevealMode"][value="direct"]')!.disabled).toBe(true);
    expect(document.querySelector<HTMLInputElement>('input[name="credentialRevealMode"][value="deny"]')!.disabled).toBe(false);
    expect(document.querySelector('.settings-security-form')).toBeNull();
  });
});
