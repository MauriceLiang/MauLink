import { describe, expect, it } from 'vitest';
import type { ConnectionSnapshot } from '../../contracts/v1/ConnectionSnapshot';
import type { SftpTransferSnapshot } from '../../contracts/v1/SftpTransferSnapshot';
import { inspectInstallActivity } from '../src/updater/install-safety';

const connection = (state: ConnectionSnapshot['state'], serverId: string): ConnectionSnapshot => ({
  connectionId: `connection-${serverId}`, serverId, mode: 'workspace', state,
  hostKeyChallenge: null, authenticationChallenge: null, negotiatedAlgorithms: null,
  error: null, createdAtMs: 0, updatedAtMs: 0,
});
const transfer = (state: SftpTransferSnapshot['state']): SftpTransferSnapshot => ({
  connectionId: 'connection-one', transferId: `transfer-${state}`, direction: 'upload',
  fileName: 'package.zip', finalPath: '/tmp/package.zip', transferredBytes: '1', totalBytes: '2',
  bytesPerSecond: null, remainingSeconds: null, state, error: null, cleanupRequired: false, temporaryPath: null,
});

describe('inspectInstallActivity', () => {
  it('treats every nonterminal SSH state as active', () => {
    const activity = inspectInstallActivity({ one: connection('awaitingCredentials', 'one'), two: connection('closed', 'two') }, [], {}, []);
    expect(activity.activeConnections.map(value => value.state)).toEqual(['awaitingCredentials']);
  });

  it('blocks active transfers and transfers whose file picker has not returned', () => {
    const activity = inspectInstallActivity({}, [transfer('finalizing'), transfer('failed')], { one: true }, []);
    expect(activity.activeTransfers).toHaveLength(1);
    expect(activity.transfersStarting).toBe(true);
  });

  it('keeps an in-flight SSH start visible until its first snapshot arrives', () => {
    const activity = inspectInstallActivity({}, [], {}, [], true);
    expect(activity.connectionsStarting).toBe(true);
    expect(activity.activeConnections).toHaveLength(0);
  });

  it('includes opening and closing terminals in the activity warning', () => {
    const activity = inspectInstallActivity({}, [], {}, [
      { id: 'opening', connectionId: 'one', state: 'opening' },
      { id: 'closing', connectionId: 'one', state: 'closing' },
      { id: 'closed', connectionId: 'one', state: 'closed' },
      { id: 'failed', connectionId: 'one', state: 'failed' },
    ]);
    expect(activity.activeTerminals.map(value => value.state)).toEqual(['opening', 'closing']);
  });
});
