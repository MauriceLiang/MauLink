import type { ApiRequest } from "../../../contracts/v1/ApiRequest";
import type { AppInfo, Command, Payload, Result } from "./commands";
import type { IpcTransport } from "./client";

export type MockHandlers = {
  [C in Command]?: (payload: Payload<C>) => Result<C> | Promise<Result<C>>;
} & { app_get_info?: () => AppInfo | Promise<AppInfo> };

export function createMockIpc(handlers: MockHandlers): IpcTransport {
  return {
    async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      if (command === "app_get_info" && handlers.app_get_info) {
        return await handlers.app_get_info() as T;
      }
      const handler = handlers[command as Command];
      if (!handler) throw new Error(`No mock handler for ${command}`);
      const request = args?.request as ApiRequest<unknown> | undefined;
      if (!request || request.apiVersion !== 1 || typeof request.requestId !== "string") {
        throw new Error("Invalid mock IPC envelope");
      }
      return await (handler as (payload: unknown) => unknown)(request.payload) as T;
    },
  };
}
