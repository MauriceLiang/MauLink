import { invoke } from "@tauri-apps/api/core";
import type { ApiRequest } from "../../../contracts/v1/ApiRequest";
import type { AppInfo, CallArgs, Command, Payload, Result } from "./commands";
import { mapError } from "../errors/mapper";

export interface IpcTransport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

export interface IpcClient {
  call<C extends Command>(command: C, ...args: CallArgs<C>): Promise<Result<C>>;
  getInfo(): Promise<AppInfo>;
}

export function createRequest<T>(payload: T): ApiRequest<T> {
  return { apiVersion: 1, requestId: crypto.randomUUID(), payload };
}

export function createIpcClient(transport: IpcTransport = { invoke }): IpcClient {
  return {
    async call<C extends Command>(command: C, ...args: CallArgs<C>): Promise<Result<C>> {
      const [payload, outputChannel] = args;
      const request = createRequest<Payload<C>>(payload);
      try {
        return await transport.invoke<Result<C>>(command, {
          request,
          ...(outputChannel === undefined ? {} : { outputChannel }),
        });
      } catch (error) {
        throw mapError(error, request.requestId);
      }
    },
    async getInfo() {
      try {
        return await transport.invoke<AppInfo>("app_get_info");
      } catch (error) {
        throw mapError(error);
      }
    },
  };
}
