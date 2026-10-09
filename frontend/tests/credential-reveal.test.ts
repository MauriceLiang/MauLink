import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import type { CredentialRevealPolicy } from '../../contracts/v1/CredentialRevealPolicy';
import type { CredentialRevealApi } from '../src/ipc/credential-reveal';
import { locale } from '../src/i18n/locale';
import CredentialRevealDialog from '../src/dialogs/CredentialRevealDialog.vue';

const wrappers: VueWrapper[] = [];
const server = { id: 'web-01', name: 'Web-01', authType: 'password' as const, hasSavedCredential: true };
function createApi(mode: CredentialRevealPolicy['mode'], reveal = vi.fn(async () => ({ kind: 'password' as const, value: 'fixture-saved-credential' }))) {
  const policy: CredentialRevealPolicy = { mode, revision: 1, hasSecondaryPassword: mode === 'protected', nativeAuthAvailable: true, nativeAuthReason: null, lockedUntilMs: null };
  const api = {
    getPolicy: vi.fn(async () => policy),
    reveal,
    enableProtected: vi.fn(), enableDirect: vi.fn(), setDeny: vi.fn(), changePassword: vi.fn(), recover: vi.fn(),
  } as unknown as CredentialRevealApi;
  return { api, reveal };
}
function mountDialog(api: CredentialRevealApi, initialServer = server) {
  const wrapper = mount(CredentialRevealDialog, { attachTo: document.body, props: { open: true, server: initialServer, api } });
  wrappers.push(wrapper);
  return wrapper;
}
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(element => element.textContent?.trim() === label)!;

afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ''; locale.value = 'zh-CN'; vi.useRealTimers(); vi.restoreAllMocks(); });

describe('saved credential reveal lifecycle', () => {
  it('keeps deny mode closed and offers navigation to Security & Privacy', async () => {
    const { api, reveal } = createApi('deny');
    const wrapper = mountDialog(api);
    await flushPromises();
    expect(document.body.textContent).toContain('当前禁止查看明文');
    expect(button('验证并查看')).toBeUndefined();
    button('打开安全与隐私').click();
    expect(wrapper.emitted('openSecurity')).toHaveLength(1);
    expect(reveal).not.toHaveBeenCalled();
  });

  it('requires the secondary password, shows one fixture value, then clears it on blur', async () => {
    const { api, reveal } = createApi('protected');
    mountDialog(api);
    await flushPromises();
    const password = document.querySelector<HTMLInputElement>('input[type="password"]')!;
    password.value = 'fixture-secondary-password';
    password.dispatchEvent(new Event('input', { bubbles: true }));
    await flushPromises();
    button('验证并查看').click();
    await flushPromises();
    expect(reveal).toHaveBeenCalledWith({ serverId: 'web-01', secondaryPassword: 'fixture-secondary-password' });
    expect(document.body.textContent).toContain('fixture-saved-credential');
    expect(document.body.textContent).toContain('15 秒后自动隐藏');
    window.dispatchEvent(new Event('blur'));
    await flushPromises();
    expect(document.body.textContent).not.toContain('fixture-saved-credential');
  });

  it('hides the displayed value after fifteen seconds', async () => {
    vi.useFakeTimers();
    const { api } = createApi('direct');
    mountDialog(api);
    await flushPromises();
    button('显示已保存凭据').click();
    await flushPromises();
    expect(document.body.textContent).toContain('fixture-saved-credential');
    await vi.advanceTimersByTimeAsync(15_000);
    await flushPromises();
    expect(document.body.textContent).not.toContain('fixture-saved-credential');
  });

  it('discards a late response after the dialog closes', async () => {
    let resolveReveal!: (value: { kind: 'password'; value: string }) => void;
    const reveal = vi.fn(() => new Promise<{ kind: 'password'; value: string }>(resolve => { resolveReveal = resolve; }));
    const { api } = createApi('direct', reveal);
    const wrapper = mountDialog(api);
    await flushPromises();
    button('显示已保存凭据').click();
    await flushPromises();
    button('关闭').click();
    await wrapper.setProps({ open: false });
    resolveReveal({ kind: 'password', value: 'late-fixture-secret' });
    await flushPromises();
    expect(document.body.textContent).not.toContain('late-fixture-secret');
  });

  it('discards a late response when the target server changes', async () => {
    let resolveReveal!: (value: { kind: 'password'; value: string }) => void;
    const reveal = vi.fn(() => new Promise<{ kind: 'password'; value: string }>(resolve => { resolveReveal = resolve; }));
    const { api } = createApi('direct', reveal);
    const wrapper = mountDialog(api);
    await flushPromises();
    button('显示已保存凭据').click();
    await flushPromises();
    await wrapper.setProps({ server: { ...server, id: 'db-01', name: 'DB-01' } });
    await flushPromises();
    resolveReveal({ kind: 'password', value: 'stale-fixture-secret' });
    await flushPromises();
    expect(document.body.textContent).not.toContain('stale-fixture-secret');
  });
});
