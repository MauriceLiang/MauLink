import type { IpcClient } from "./client";
import type { CallArgs, Payload } from "./commands";

export function createTerminalApi(client: IpcClient) {
  return {
    open: (...args: CallArgs<"terminal_open">) => client.call("terminal_open", ...args),
    get: (payload: Payload<"terminal_get">) => client.call("terminal_get", payload),
    write: (payload: Payload<"terminal_write">) => client.call("terminal_write", payload),
    resize: (payload: Payload<"terminal_resize">) => client.call("terminal_resize", payload),
    ack: (payload: Payload<"terminal_ack">) => client.call("terminal_ack", payload),
    close: (payload: Payload<"terminal_close">) => client.call("terminal_close", payload),
  };
}
