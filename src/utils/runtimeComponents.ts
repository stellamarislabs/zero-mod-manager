import type { ModSummary } from "../types";

// A view-only heuristic based on deployed folders, never on editable display names.
const bundledFolders = new Set([
  "bpmodloadermod", "consolecommandsmod", "consoleenablermod",
  "cheatmanagerenablermod", "keybinds", "splitscreenmod", "linetracemod",
  "bpml_genericfunctions",
]);

export function isUe4ssComponent(mod: ModSummary): boolean {
  return mod.modType === "ue4ss" && mod.files.length > 0 && mod.files.every(file => {
    const path = file.destination.replace(/\\/g, "/");
    const folder = path.match(/(?:^|\/)ue4ss\/mods\/([^/]+)\//i)?.[1];
    return !!folder && bundledFolders.has(folder.toLowerCase());
  });
}
