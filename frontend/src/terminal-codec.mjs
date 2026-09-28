export function decodeBase64(value) {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

export function encodeBytesBase64(bytes) {
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return btoa(binary);
}

export function encodeTextBase64(value) {
  return encodeBytesBase64(new TextEncoder().encode(value));
}

export function createTerminalOutputConsumer(write, acknowledge) {
  let queue = Promise.resolve();
  let nextSequence = 1n;

  return (chunk) => {
    queue = queue.then(async () => {
      if (typeof chunk.seq !== "string" || !/^\d+$/.test(chunk.seq) || BigInt(chunk.seq) !== nextSequence) {
        throw new Error("终端输出序号不连续，已停止接收。");
      }
      const bytes = decodeBase64(chunk.dataBase64);
      if (bytes.byteLength !== chunk.byteLength) {
        throw new Error("终端数据长度校验失败，已停止接收。");
      }
      await new Promise((resolve, reject) => {
        try {
          write(bytes, resolve);
        } catch (error) {
          reject(error);
        }
      });
      await acknowledge(chunk);
      nextSequence += 1n;
    });
    return queue;
  };
}
