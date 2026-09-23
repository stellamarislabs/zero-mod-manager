import type { ModSummary } from "../types";

export function groupMods(mods: ModSummary[]) {
  const groups = new Map<string, ModSummary[]>();
  for (const mod of mods) {
    const key = mod.bundleId ? `bundle:${mod.bundleId}` : `mod:${mod.id}`;
    groups.set(key, [...(groups.get(key) ?? []), mod]);
  }
  return [...groups].map(([id, members]) => ({ id, members, name: members.find(mod => mod.bundleName)?.bundleName ?? members[0].name }));
}

/** Do not invent a shared version when bundle components differ. */
export function installedVersion(members: ModSummary[]): string {
  const versions = [...new Set(members.map(mod => mod.version?.trim() || null))];
  if (versions.length > 1) return "Mixed versions · see components";
  return versions[0] ? `Installed v${versions[0]}` : "Version not recorded";
}
