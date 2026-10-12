import type { ConnectionSnapshot } from '../../../contracts/v1/ConnectionSnapshot';
import type { SftpTransferSnapshot } from '../../../contracts/v1/SftpTransferSnapshot';
import type { TerminalState } from '../../../contracts/v1/TerminalState';
import { isFinished } from '../stores/connections';
import { transferFinished } from '../stores/transfers';

export interface InstallActivity {
  activeConnections: ConnectionSnapshot[];
  connectionsStarting: boolean;
  activeTransfers: SftpTransferSnapshot[];
  transfersStarting: boolean;
  activeTerminals: { id: string; connectionId: string; state: TerminalState }[];
}

export function inspectInstallActivity(
  connections: Record<string, ConnectionSnapshot>,
  transfers: SftpTransferSnapshot[],
  starting: Record<string, boolean>,
  terminals: { id: string; connectionId: string; state: TerminalState }[],
  connectionsStarting = false,
): InstallActivity {
  return {
    activeConnections: Object.values(connections).filter(snapshot => !isFinished(snapshot)),
    connectionsStarting,
    activeTransfers: transfers.filter(task => !transferFinished(task)),
    transfersStarting: Object.values(starting).some(Boolean),
    activeTerminals: terminals.filter(tab => tab.state !== 'closed' && tab.state !== 'failed'),
  };
}
