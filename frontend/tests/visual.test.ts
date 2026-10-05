import { describe, expect, it, vi } from 'vitest';
import type { Channel } from '@tauri-apps/api/core';
import type { TerminalChunk } from '../../contracts/v1/TerminalChunk';
import { createIpcClient } from '../src/ipc/client';
import { createTerminalApi } from '../src/ipc/terminal';
import { createVisualMock, createVisualStorage, visualConfig, visualEpoch, visualFiles } from '../src/harness/visual-fixtures';
import cases from '../visual/cases.json';
const config = (page = 'servers') => ({page,theme:'dark' as const,locale:'en' as const});
describe('deterministic visual fixtures', () => {
  it('validates stable page/theme/locale inputs and falls back safely', () => {
    expect(new Set(cases.map(item=>item.page)).size).toBe(cases.length);
    expect(visualConfig(new URLSearchParams('page=host-key&theme=dark&locale=en'))).toEqual(config('host-key'));
    expect(visualConfig(new URLSearchParams('page=https://example.test/&theme=unknown&locale=unknown'))).toEqual({page:'servers',theme:'light',locale:'zh-CN'});
  });
  it('includes the Server Overview and terminal personalization acceptance scenes', () => {
    const pages = new Set(cases.map(item=>item.page));
    for (const page of ['server-overview','terminal-light','terminal-dark','terminal-custom','terminal-image','settings-terminal-image']) expect(pages.has(page)).toBe(true);
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
});
