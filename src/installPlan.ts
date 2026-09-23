import type { ModPreview, ModSummary } from "./types";

/** A shared file/folder is replacement evidence, not evidence of a bundle. */
export function replacementBundle(previews: ModPreview[], mods: ModSummary[]): string | null {
  const replacements = previews.flatMap(preview => preview.replaces ? [preview.replaces.modId] : []);
  if (!replacements.length) return null;
  const owners = replacements.map(id => mods.find(mod => mod.id === id));
  const bundle = owners[0]?.bundleId;
  return bundle && owners.every(owner => owner?.bundleId === bundle) ? bundle : null;
}

/** Removing a Library entry also removes its pending replacement claim. */
export function reconcilePreviews(previews: ModPreview[], mods: ModSummary[]): ModPreview[] {
  const ids = new Set(mods.map(mod => mod.id));
  return previews.map(preview => ({
    ...preview,
    replaces: preview.replaces && ids.has(preview.replaces.modId) ? preview.replaces : null,
    conflicts: preview.conflicts.filter(conflict => ids.has(conflict.modId)),
  }));
}
