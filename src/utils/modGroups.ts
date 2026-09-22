import type { ModSummary } from "../types";

export function groupMods(mods: ModSummary[]) {
  const groups = new Map<string, ModSummary[]>();
  for (const mod of mods) {
    const key = mod.bundleId ? `bundle:${mod.bundleId}` : `mod:${mod.id}`;
    groups.set(key, [...(groups.get(key) ?? []), mod]);
  }
  return [...groups].map(([id, members]) => ({ id, members, name: members[0].name }));
}
