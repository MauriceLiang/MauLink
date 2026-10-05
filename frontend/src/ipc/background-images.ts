import { convertFileSrc } from "@tauri-apps/api/core";
import type { BackgroundImageAsset } from "../../../contracts/v1/BackgroundImageAsset";
import type { BackgroundImagePayload } from "../../../contracts/v1/BackgroundImagePayload";
import type { IpcClient } from "./client";

export function createBackgroundImagesApi(
  client: IpcClient,
  assetUrl: (path: string) => string = convertFileSrc,
) {
  return {
    async select() {
      const selected = await client.call("local_file_select", { purpose: "terminalBackground" });
      if (!selected) return null;
      return client.call("background_image_import", { token: selected.token });
    },
    get: (imageId: string) => client.call("background_image_get", { imageId } satisfies BackgroundImagePayload),
    delete: (imageId: string) => client.call("background_image_delete", { imageId } satisfies BackgroundImagePayload),
    async resolve(imageId: string): Promise<{ asset: BackgroundImageAsset; src: string }> {
      const result = await client.call("background_image_get", { imageId });
      return { asset: result.asset, src: assetUrl(result.localPath) };
    },
  };
}

export type BackgroundImagesApi = ReturnType<typeof createBackgroundImagesApi>;
