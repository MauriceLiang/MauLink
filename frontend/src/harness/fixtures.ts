import type { AppError } from "../../../contracts/v1/AppError";
import type { ServerListPage } from "../../../contracts/v1/ServerListPage";
import { createMockIpc } from "../ipc/mock";

export const emptyServers: ServerListPage = { items: [], nextCursor: null };
export const timeoutError: AppError = {
  code: "CONNECTION_TIMEOUT", messageKey: "errors.connectionTimeout", params: {},
  retryable: true, action: "retry", stage: "connection", requestId: null, details: null,
};

export function createFoundationMock() {
  return createMockIpc({
    server_list: payload => {
      if (payload.query === "error") throw timeoutError;
      return emptyServers;
    },
  });
}
