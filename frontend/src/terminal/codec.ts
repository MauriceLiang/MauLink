import type { TerminalChunk } from "../../../contracts/v1/TerminalChunk";
export function decodeBase64(value: string) {
  const binary = atob(value);
  return Uint8Array.from(binary, character => character.charCodeAt(0));
}
export function encodeBytesBase64(bytes: Uint8Array) {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 0x8000) binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  return btoa(binary);
}

// Decimal strings preserve the u64 contract on WebViews without BigInt support.
export function nextSequence(value: string) {
  const digits = value.split('');
  for (let index = digits.length - 1; index >= 0; index--) {
    if (digits[index] !== '9') { digits[index] = String(Number(digits[index]) + 1); return digits.join(''); }
    digits[index] = '0';
  }
  return '1' + digits.join('');
}

export function createOutputConsumer(write: (bytes: Uint8Array, done: () => void) => void, ack: (chunk: TerminalChunk) => Promise<void>) {
  let queue = Promise.resolve();
  let sequence = '1';
  let pendingBytes = 0;
  let stopped = false;
  return (chunk: TerminalChunk) => {
    if (stopped) return Promise.reject(new Error('终端输出已停止。'));
    if (!Number.isSafeInteger(chunk.byteLength) || chunk.byteLength < 0 || chunk.byteLength > 32768 || pendingBytes + chunk.byteLength > 131072) {
      stopped = true;
      return Promise.reject(new Error('终端输出超出流控窗口。'));
    }
    pendingBytes += chunk.byteLength;
    let accounted = true;
    const release = () => { if (accounted) { pendingBytes -= chunk.byteLength; accounted = false; } };
    queue = queue.then(async () => {
      if (stopped) throw new Error('终端输出已停止。');
      if (typeof chunk.seq !== 'string' || !/^\d+$/.test(chunk.seq) || chunk.seq.replace(/^0+(?=\d)/, '') !== sequence) throw new Error('终端输出序号不连续。');
      const bytes = decodeBase64(chunk.dataBase64);
      if (bytes.byteLength !== chunk.byteLength) throw new Error('终端数据长度校验失败。');
      await new Promise<void>((resolve, reject) => { try { write(bytes, resolve); } catch (error) { reject(error); } });
      // Core can send the next chunk before the ACK response reaches this WebView.
      // These bytes have already been rendered; only unwritten bytes occupy our queue.
      release();
      await ack(chunk);
      sequence = nextSequence(sequence);
    }).catch(error => { stopped = true; throw error; }).finally(release);
    return queue;
  };
}
