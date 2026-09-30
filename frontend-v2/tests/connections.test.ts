import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { createConnectionStore } from "../src/stores/connections";
import { createConnectionApi } from "../src/ipc/connection";
import { createIpcClient } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createConnectionMock } from "../src/harness/connection-fixtures";
import { fixtureError } from "../src/harness/server-fixtures";
import { shellServers } from "../src/harness/shell-fixtures";
import ConnectionDialogs from "../src/dialogs/ConnectionDialogs.vue";
import AppShell from "../src/app/AppShell.vue";
import ConnectionPanel from "../src/components/connection/ConnectionPanel.vue";
import type { ConnectionSnapshot } from "../../contracts/v1/ConnectionSnapshot";
const server = shellServers[0]!;
const stores: ReturnType<typeof createConnectionStore>[] = [];
const wrappers: VueWrapper[] = [];
function storeFor(transport = createConnectionMock(() => 'unknown').transport) { const store = createConnectionStore(createConnectionApi(createIpcClient(transport))); stores.push(store); return store; }
const initial: ConnectionSnapshot = { connectionId: 'connection-a', serverId: server.id, mode: 'workspace', state: 'connecting', hostKeyChallenge: null, authenticationChallenge: null, negotiatedAlgorithms: null, error: null, createdAtMs: 1, updatedAtMs: 1 };
function button(wrapper: VueWrapper, text: string) { return wrapper.findAll('button').find(button => button.text() === text)!; }
function dialogs(store: ReturnType<typeof storeFor>) { const wrapper = mount(ConnectionDialogs, { attachTo: document.body, props: { store, servers: [server] }, global: { stubs: { Teleport: true } } }); wrappers.push(wrapper); return wrapper; }
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); stores.splice(0).forEach(store => store.dispose()); vi.useRealTimers(); document.body.innerHTML = ''; });

describe('secure connection flow', () => {
  it('starts with saved revision/workspace mode, blocks duplicate starts and disposes polling', async () => {
    vi.useFakeTimers(); const start = vi.fn(() => initial); const get = vi.fn(() => ({ ...initial, state: 'ready' as const }));
    const store = storeFor(createMockIpc({ connection_start: start, connection_get: get }));
    await Promise.all([store.start(server), store.start(server)]);
    expect(start).toHaveBeenCalledOnce(); expect(start).toHaveBeenCalledWith({ source: { kind: 'saved', serverId: server.id, expectedRevision: server.revision }, mode: 'workspace' });
    await vi.advanceTimersByTimeAsync(300); expect(store.snapshots.value[server.id]?.state).toBe('ready');
    store.dispose(); await vi.advanceTimersByTimeAsync(3000); expect(get).toHaveBeenCalledOnce();
  });
  it('does not allow a late poll to undo a successful native cancel', async () => {
    let resolve!: (value: ConnectionSnapshot) => void;
    const cancel = vi.fn(() => ({ ...initial, state: 'cancelled' as const }));
    const store = storeFor(createMockIpc({ connection_start: () => initial, connection_get: () => new Promise<ConnectionSnapshot>(done => { resolve = done; }), connection_cancel: cancel }));
    await store.start(server); const polling = store.refresh(server.id); await store.cancel(server.id);
    resolve({ ...initial, state: 'awaitingHostTrust' }); await polling;
    expect(store.snapshots.value[server.id]?.state).toBe('cancelled'); expect(cancel).toHaveBeenCalledWith({ connectionId: initial.connectionId });
  });
  it('never responds automatically; first host identity exposes endpoint, algorithm, fingerprint and Esc rejects', async () => {
    const mock = createConnectionMock(() => 'unknown'); const store = storeFor(mock.transport); await store.start(server); await store.refresh(server.id);
    const wrapper = dialogs(store); await flushPromises();
    expect(mock.calls).not.toContain('host_key_respond');
    expect(wrapper.text()).toContain('fixture.example.test:22'); expect(wrapper.text()).toContain('ssh-ed25519'); expect(wrapper.text()).toContain('SHA256:phase5-current-fingerprint');
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Escape' }); await flushPromises(); await store.refresh(server.id);
    expect(mock.calls.filter(call => call === 'host_key_respond')).toHaveLength(1); expect(store.snapshots.value[server.id]?.error?.code).toBe('HOST_KEY_REJECTED');
  });
  it('changed keys default to rejection, require independent verification to update, and never offer trustOnce', async () => {
    const mock = createConnectionMock(() => 'changed'); const store = storeFor(mock.transport); await store.start(server); await store.refresh(server.id);
    const wrapper = dialogs(store); await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain('未核实'); expect(wrapper.text()).toContain('SHA256:phase5-previous-fingerprint'); expect(wrapper.text()).not.toContain('仅本次信任');
    expect(button(wrapper, '更新记录并连接')).toBeUndefined(); await button(wrapper, '高级：更新信任记录').trigger('click');
    expect(button(wrapper, '更新记录并连接').element).toHaveProperty('disabled', true);
    await wrapper.get('input[type="checkbox"]').setValue(true); await button(wrapper, '更新记录并连接').trigger('click'); await flushPromises();
    expect(mock.calls.filter(call => call === 'host_key_respond')).toHaveLength(1);
    await store.refresh(server.id); expect(store.snapshots.value[server.id]?.state).toBe('awaitingCredentials');
  });
  it('busy challenge Esc does not respond or cancel a second time', async () => {
    let resolve!: () => void;
    const response = vi.fn(() => new Promise<void>(done => { resolve = done; }));
    const host = { challengeId: 'challenge-a', connectionId: initial.connectionId, host: 'host.test', port: 22, algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:current', previousFingerprintSha256: null, previousRevision: null, expiresAtMs: Date.now()+120000 };
    const store = storeFor(createMockIpc({ connection_start: () => ({ ...initial, state: 'awaitingHostTrust', hostKeyChallenge: host }), host_key_respond: response }));
    await store.start(server); const wrapper = dialogs(store); await flushPromises();
    await button(wrapper, '信任并保存').trigger('click'); await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Escape' });
    expect(response).toHaveBeenCalledOnce(); expect(wrapper.get('[role="dialog"]').attributes('aria-busy')).toBe('true');
    resolve(); await flushPromises(); expect(wrapper.find('[role="dialog"]').exists()).toBe(false);
  });
  it('authentication uses challenge ids and transient masked input; wrong password is understandable without retry', async () => {
    const mock = createConnectionMock(() => 'password'); const store = storeFor(mock.transport); await store.start(server); await store.refresh(server.id);
    const wrapper = dialogs(store); await flushPromises(); expect(wrapper.get('input').attributes('type')).toBe('password');
    await wrapper.get('form').trigger('submit'); expect(wrapper.text()).toContain('请输入凭据');
    await wrapper.get('input').setValue('wrong'); await wrapper.get('form').trigger('submit'); await flushPromises(); expect(wrapper.find('[role="dialog"]').exists()).toBe(false);
    expect(JSON.stringify(store.snapshots.value)).not.toContain('wrong'); await store.refresh(server.id);
    const panel = mount(ConnectionPanel, { props: { store, server, readOnly: false } }); wrappers.push(panel);
    expect(panel.text()).toContain('认证失败'); expect(button(panel, '重试连接')).toBeUndefined(); expect(panel.text()).not.toContain('Fixture debug');
    await button(panel, '关闭错误').trigger('click'); expect(button(panel, '连接')).toBeDefined();
  });
  it('clears the credential input immediately while a response is pending and keeps auth payload out of Store', async () => {
    let resolve!: () => void;
    const response = vi.fn(() => new Promise<void>(done => { resolve = done; }));
    const auth = { challengeId: 'auth-a', connectionId: initial.connectionId, credentialKind: 'password' as const, expiresAtMs: Date.now()+120000 };
    const store = storeFor(createMockIpc({ connection_start: () => ({ ...initial, state: 'awaitingCredentials', authenticationChallenge: auth }), auth_respond: response }));
    await store.start(server); const wrapper = dialogs(store); await flushPromises(); await wrapper.get('input').setValue('fixture-transient');
    await wrapper.get('form').trigger('submit'); expect(wrapper.get('input').element).toHaveProperty('value','');
    expect(response).toHaveBeenCalledWith({ connectionId: initial.connectionId, challengeId: auth.challengeId, secret: 'fixture-transient' });
    expect(JSON.stringify(store.snapshots.value)).not.toContain('fixture-transient'); resolve(); await flushPromises();
  });
  it('authentication Esc invokes cancel; private key passphrase challenge uses the same Core flow', async () => {
    const mock = createConnectionMock(() => 'passphrase'); const store = storeFor(mock.transport); await store.start(server); await store.refresh(server.id);
    const wrapper = dialogs(store); await flushPromises(); expect(wrapper.text()).toContain('输入私钥口令');
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Escape' }); await flushPromises();
    expect(mock.calls).toContain('connection_cancel'); expect(store.snapshots.value[server.id]?.state).toBe('cancelled');
  });
  it('disconnects ready connections without implicitly stopping transfers', async () => {
    const disconnect = vi.fn(() => ({ ...initial, state: 'closed' as const })); const cancel = vi.fn();
    const store = storeFor(createMockIpc({ connection_start: () => ({ ...initial, state: 'ready' }), connection_disconnect: disconnect, connection_cancel: cancel }));
    await store.start(server); await store.cancel(server.id); expect(cancel).not.toHaveBeenCalled();
    await store.disconnect(server.id); expect(disconnect).toHaveBeenCalledWith({ connectionId: initial.connectionId, stopActiveTransfers: false }); expect(store.snapshots.value[server.id]?.state).toBe('closed');
  });
  it('shows retry only for retryable terminal errors; host-key changes stay blocked even with retryable true', async () => {
    const store = storeFor(); const error = { ...fixtureError('CONNECTION_TIMEOUT','errors.connectionTimeout'), retryable: true, stage: 'connectingProxy', requestId: 'safe-request-id' };
    store.snapshots.value = { [server.id]: { ...initial, state: 'failed', error } };
    const panel = mount(ConnectionPanel, { props: { store, server, readOnly: false } }); wrappers.push(panel);
    expect(button(panel,'重试连接')).toBeDefined(); expect(panel.text()).not.toContain('safe-request-id');
    await button(panel,'查看诊断').trigger('click'); expect(panel.text()).toContain('safe-request-id'); expect(panel.text()).not.toContain('Fixture debug');
    store.snapshots.value = { [server.id]: { ...initial, state: 'failed', error: { ...error, code: 'HOST_KEY_CHANGED' } } }; await flushPromises(); expect(button(panel,'重试连接')).toBeUndefined();
  });
  it('retains the last Core snapshot on polling failure and permits real cancellation', async () => {
    const store = storeFor(createMockIpc({ connection_start: () => initial, connection_get: () => { throw new Error('secret raw transport'); }, connection_cancel: () => ({ ...initial, state: 'cancelled' }) }));
    await store.start(server); await store.refresh(server.id); expect(store.snapshots.value[server.id]).toEqual(initial); expect(store.errors.value[server.id]?.details).toBeNull();
    await store.cancel(server.id); expect(store.snapshots.value[server.id]?.state).toBe('cancelled');
  });
  it('restores focus to a current action when a security dialog closes and its original trigger disappeared', async () => {
    const mock = createConnectionMock(() => 'changed');
    const wrapper = mount(AppShell, { attachTo: document.body, props: { client: createIpcClient(mock.transport) }, global: { stubs: { Teleport: true } } }); wrappers.push(wrapper);
    await flushPromises(); await wrapper.get('[aria-label="查看 Web-01"]').trigger('click');
    button(wrapper, '连接').element.focus(); await button(wrapper, '连接').trigger('click'); await flushPromises();
    await vi.waitFor(() => expect(wrapper.find('[role="dialog"]').exists()).toBe(true));
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Escape' }); await flushPromises();
    await vi.waitFor(() => expect(button(wrapper, '关闭错误')).toBeDefined());
    await flushPromises();
    expect(document.activeElement).toBe(button(wrapper, '关闭错误').element);
  });
  it('preserves expired response errors while later successful polls still show the same pending challenge', async () => {
    const mock = createConnectionMock(() => 'unknown'); const store = storeFor({ invoke: async <T,>(command: string, args?: Record<string, unknown>) => { if(command === 'host_key_respond') throw fixtureError('CHALLENGE_EXPIRED','errors.challengeExpired'); return mock.transport.invoke<T>(command,args); } });
    await store.start(server); await store.refresh(server.id); await store.respondHostKey(server.id,'trustOnce'); await store.refresh(server.id);
    expect(store.errors.value[server.id]?.code).toBe('CHALLENGE_EXPIRED'); expect(store.challenge.value).toBeDefined();
  });
});
