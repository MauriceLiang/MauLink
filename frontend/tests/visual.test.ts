import { describe, expect, it, vi } from 'vitest';
import type { Channel } from '@tauri-apps/api/core';
import type { TerminalChunk } from '../../contracts/v1/TerminalChunk';
import { createIpcClient } from '../src/ipc/client';
import { createTerminalApi } from '../src/ipc/terminal';
import { createVisualMock, createVisualStorage, visualConfig, visualEpoch, visualFiles, visualTransfers } from '../src/harness/visual-fixtures';
import cases from '../visual/cases.json';
import casesV33 from '../visual/cases-v33.json';
import matrixV33 from '../visual/matrix-v33.json';
const config = (page = 'servers') => ({page,theme:'dark' as const,locale:'en' as const,accentColor:'blue' as const,customAccentColor:null,uiDensity:'standard' as const});
describe('deterministic visual fixtures', () => {
  it('validates stable page/theme/locale/customization inputs and falls back safely', () => {
    expect(new Set(cases.map(item=>item.page)).size).toBe(cases.length);
    expect(visualConfig(new URLSearchParams('page=host-key&theme=dark&locale=en'))).toEqual(config('host-key'));
    expect(visualConfig(new URLSearchParams('page=servers&theme=system'))).toMatchObject({page:'servers',theme:'system'});
    expect(visualConfig(new URLSearchParams('page=settings-appearance&theme=light&accentColor=custom&customAccentColor=%23D92D20&uiDensity=compact'))).toMatchObject({page:'settings-appearance',theme:'light',accentColor:'custom',customAccentColor:'#D92D20',uiDensity:'compact'});
    expect(visualConfig(new URLSearchParams('page=https://example.test/&theme=unknown&locale=unknown&accentColor=invalid&customAccentColor=red&uiDensity=invalid'))).toEqual({page:'servers',theme:'light',locale:'zh-CN',accentColor:'blue',customAccentColor:null,uiDensity:'standard'});
  });
  it('includes the Server Overview and terminal personalization acceptance scenes', () => {
    const pages = new Set(cases.map(item=>item.page));
    for (const page of ['server-overview','terminal-light','terminal-dark','terminal-custom','terminal-image','settings-terminal-image','settings-security','delete-group','disconnect','disconnect-transfer','quick-monitor','credential-reveal']) expect(pages.has(page)).toBe(true);
  });
  it('covers the V3.3 viewport, theme, locale, density and accent matrix without touching legacy routes', () => {
    const pages = new Set([...cases, ...casesV33].map(item => item.page));
    const ids = matrixV33.map(item => item.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect([...new Set(matrixV33.map(item => `${item.width}x${item.height}`))].sort()).toEqual(['1440x920', '860x640', '988x756']);
    expect(new Set(matrixV33.map(item => item.theme))).toEqual(new Set(['dark', 'light', 'system']));
    expect(matrixV33.some(item => item.locale === 'en')).toBe(true);
    expect(matrixV33.some(item => item.uiDensity === 'compact')).toBe(true);
    expect(matrixV33.some(item => item.accentColor === 'red')).toBe(true);
    expect(matrixV33.some(item => item.accentColor === 'custom' && item.customAccentColor === '#0057FF')).toBe(true);
    for (const item of matrixV33) for (const page of item.pages ?? [...cases, ...casesV33].map(value => value.page)) expect(pages.has(page)).toBe(true);
    expect(cases.map(item => item.page)).toHaveLength(36);
  });
  it('covers V3.3 edge states with deterministic server, SFTP, transfer and monitor fixtures', async () => {
    expect(new Set(casesV33.map(item=>item.page)).size).toBe(casesV33.length);
    expect(visualConfig(new URLSearchParams('page=tree-50'))).toMatchObject({ page: 'tree-50' });
    const treeClient=createIpcClient(createVisualMock(config('tree-50')));
    expect((await treeClient.call('server_list',{limit:100,cursor:null,query:null,groupId:null})).items).toHaveLength(50);

    const pagedClient=createIpcClient(createVisualMock(config('files-paginated')));
    const first=await pagedClient.call('sftp_list_start',{connectionId:'visual-connection',path:'/opt/app'});
    expect(first.entries).toHaveLength(10);
    const second=await pagedClient.call('sftp_list_next',{cursorId:first.cursorId!});
    expect(second.entries[0]?.name).toBe('page-item-011.txt');
    const emptyClient=createIpcClient(createVisualMock(config('files-empty')));
    expect((await emptyClient.call('sftp_list_start',{connectionId:'visual-connection',path:'/opt/app/empty'})).entries).toEqual([]);
    const directoryClient=createIpcClient(createVisualMock(config('files-directory-open')));
    expect((await directoryClient.call('sftp_list_start',{connectionId:'visual-connection',path:'/opt/app/src'})).entries.map(entry=>entry.name)).toEqual(['App.vue','components']);

    const staleClient=createIpcClient(createVisualMock(config('monitor-stale')));
    const stale=await staleClient.call('monitor_get_snapshot',{connectionId:'visual-connection'});
    expect(stale.cpu.quality.status).toBe('stale');
    const errorClient=createIpcClient(createVisualMock(config('monitor-error')));
    const error=await errorClient.call('monitor_get_snapshot',{connectionId:'visual-connection'});
    expect(error.cpu.quality.status).toBe('error');
    expect(visualTransfers('visual-connection').some(task=>task.state==='cancelled')).toBe(true);
    const multiServerTasks = visualTransfers('visual-connection').concat(visualTransfers('visual-connection-db-01-1','visual-transfer-db-01').slice(0,1));
    expect(new Set(multiServerTasks.map(task=>task.transferId)).size).toBe(multiServerTasks.length);
  });
  it('returns equal visible state under different wall clocks and independent mounts', async () => {
    async function capture() {
      const client=createIpcClient(createVisualMock(config('transfer')));
      return Promise.all([client.call('server_list',{limit:100,cursor:null,query:null,groupId:null}),client.call('settings_get',{}),client.call('sftp_list_start',{connectionId:'visual-connection',path:'.'}),client.call('sftp_transfer_list',{connectionId:'visual-connection',limit:50}),client.call('monitor_get_snapshot',{connectionId:'visual-connection'}),client.call('monitor_get_history',{connectionId:'visual-connection',metric:'cpuUsage',fromMs:Date.now()-120000,toMs:Date.now(),limit:120})]);
    }
    const before=await capture(); vi.spyOn(Date,'now').mockReturnValue(4102444800000); expect(await capture()).toEqual(before);
    expect(visualFiles.every(file=>file.modifiedAtMs===visualEpoch)).toBe(true);
  });
  it('keeps transfer progress fixed across polls and isolates empty home', async () => {
    const client=createIpcClient(createVisualMock(config('transfer'))); const [task]=await client.call('sftp_transfer_list',{connectionId:'visual-connection',limit:50});
    expect(await client.call('sftp_transfer_get',{transferId:task!.transferId})).toEqual(task);
    expect((await createIpcClient(createVisualMock(config('empty'))).call('server_list',{limit:100,cursor:null,query:null,groupId:null})).items).toEqual([]);
  });
  it.each(['host-key','host-key-changed','authentication','connection-error'])('returns a stable %s challenge/error', async page => {
    const client=createIpcClient(createVisualMock(config(page)));
    const snapshot=await client.call('connection_start',{source:{kind:'saved',serverId:'web-01',expectedRevision:1},mode:'workspace'});
    expect(await client.call('connection_get',{connectionId:snapshot.connectionId})).toEqual(snapshot);
    expect(snapshot.state).toBe(page==='authentication'?'awaitingCredentials':page==='connection-error'?'failed':'awaitingHostTrust');
    expect(snapshot.createdAtMs).toBe(visualEpoch);
  });
  it('assigns a fresh connection ID when a server reconnects after disconnect', async () => {
    const client=createIpcClient(createVisualMock(config('workspace-tabs')));
    const source={kind:'saved' as const,serverId:'web-01',expectedRevision:1};
    const first=await client.call('connection_start',{source,mode:'workspace'});
    await client.call('connection_disconnect',{connectionId:first.connectionId,stopActiveTransfers:false});
    const second=await client.call('connection_start',{source,mode:'workspace'});
    expect(second.connectionId).not.toBe(first.connectionId);
  });
  it('feeds fixed UTF-8 output through the real Channel boundary before open resolves', async () => {
    const client=createIpcClient(createVisualMock(config('terminal'))); const received=vi.fn();
    const channel={onmessage:received} as unknown as Channel<TerminalChunk>;
    const opened=await createTerminalApi(client).open({connectionId:'visual-connection',columns:80,rows:24,pixelWidth:null,pixelHeight:null},channel);
    const [chunk]=received.mock.calls[0]!;
    expect(chunk).toMatchObject({...opened,seq:'1'});
    expect(atob(chunk.dataBase64)).toContain('Release 2.14.0 activated');
    expect((await client.call('terminal_get',{terminalId:opened.terminalId})).state).toBe('running');
  });
  it('isolates copy-on-select preferences between fresh visual documents', () => {
    const first=createVisualStorage(),second=createVisualStorage();
    first.setItem('maulink.terminal.copyOnSelect','true');
    expect(first.getItem('maulink.terminal.copyOnSelect')).toBe('true');
    expect(second.getItem('maulink.terminal.copyOnSelect')).toBeNull();
    expect(first.length).toBe(1);first.clear();expect(first.length).toBe(0);
  });
  it('rejects unsupported writes instead of falling through to native IPC', async () => {
    const client=createIpcClient(createVisualMock(config()));
    await expect(client.call('sftp_delete',{connectionId:'visual-connection',path:'/opt/app/package.json',expectedType:'file',confirmed:true})).rejects.toMatchObject({code:'INTERNAL'});
  });
  it('provides an available default credential viewing policy for the security screen', async () => {
    const client=createIpcClient(createVisualMock(config('settings-security')));
    expect(await client.call('reveal_policy_get',{})).toMatchObject({mode:'deny',revision:1,nativeAuthAvailable:true});
  });
});
