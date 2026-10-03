import type { IpcClient } from "./client";
import type { CallArgs, Payload } from "./commands";

export function createSftpApi(client: IpcClient) {
  return {
    listStart: (payload: Payload<"sftp_list_start">) => client.call("sftp_list_start", payload),
    listNext: (payload: Payload<"sftp_list_next">) => client.call("sftp_list_next", payload),
    listClose: (payload: Payload<"sftp_list_close">) => client.call("sftp_list_close", payload),
    stat: (payload: Payload<"sftp_stat">) => client.call("sftp_stat", payload),
    readText: (payload: Payload<"sftp_read_text">) => client.call("sftp_read_text", payload),
    writeText: (payload: Payload<"sftp_write_text">) => client.call("sftp_write_text", payload),
    writeTextWithSudo: (payload: Payload<"sftp_write_text_with_sudo">) => client.call("sftp_write_text_with_sudo", payload),
    mkdir: (payload: Payload<"sftp_mkdir">) => client.call("sftp_mkdir", payload),
    rename: (payload: Payload<"sftp_rename">) => client.call("sftp_rename", payload),
    remove: (payload: Payload<"sftp_delete">) => client.call("sftp_delete", payload),
    upload: (...args: CallArgs<"sftp_upload">) => client.call("sftp_upload", ...args),
    download: (...args: CallArgs<"sftp_download">) => client.call("sftp_download", ...args),
    getTransfer: (payload: Payload<"sftp_transfer_get">) => client.call("sftp_transfer_get", payload),
    listTransfers: (payload: Payload<"sftp_transfer_list">) => client.call("sftp_transfer_list", payload),
    cancelTransfer: (payload: Payload<"sftp_transfer_cancel">) => client.call("sftp_transfer_cancel", payload),
  };
}
