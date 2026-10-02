import type { ServerProfile } from "../../../contracts/v1/ServerProfile";

export type ServerNavigationAction = "workspace" | "monitor" | "files" | "edit" | "remove";
export type ServerNavigationRequest = { action: ServerNavigationAction; server: ServerProfile };
