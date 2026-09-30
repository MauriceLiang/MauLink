const COPY_ON_SELECT_KEY = "maulink.terminal.copyOnSelect";

export function isTerminalCopyOnSelectEnabled(storage) {
  return storage.getItem(COPY_ON_SELECT_KEY) === "true";
}

export function setTerminalCopyOnSelectEnabled(enabled, storage) {
  storage.setItem(COPY_ON_SELECT_KEY, enabled ? "true" : "false");
}

export async function copyTerminalSelection(selection, enabled, clipboard) {
  if (!enabled || !selection) return false;
  if (typeof clipboard?.writeText !== "function") throw new Error("Clipboard API unavailable.");
  await clipboard.writeText(selection);
  return true;
}
