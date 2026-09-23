import { describe, expect, it } from "vitest";
import type { ModPreview, ModSummary } from "./types";
import { reconcilePreviews, replacementBundle } from "./installPlan";
import { installedVersion } from "./utils/modGroups";

const mod = (id: string, bundleId: string | null = null, version: string | null = "1.0") => ({ id, bundleId, version }) as ModSummary;
const preview = (id: string | null) => ({ replaces: id ? { modId: id } : null, conflicts: [] }) as unknown as ModPreview;

describe("replacement planning", () => {
  it("never sends a standalone update into the bundle endpoint", () => {
    expect(replacementBundle([preview("a")], [mod("a")])).toBeNull();
  });
  it("only offers complete updates for one explicit bundle", () => {
    expect(replacementBundle([preview("a"), preview("b"), preview(null)], [mod("a", "bundle"), mod("b", "bundle")])).toBe("bundle");
    expect(replacementBundle([preview("a"), preview("b")], [mod("a", "bundle"), mod("b")])).toBeNull();
    expect(replacementBundle([preview("a"), preview("b")], [mod("a", "one"), mod("b", "two")])).toBeNull();
  });
  it("uninstall clears replacement and conflict claims in an open preview", () => {
    const original = { ...preview("deleted"), conflicts: [{ modId: "deleted" }] } as ModPreview;
    const result = reconcilePreviews([original], []);
    expect(result[0].replaces).toBeNull();
    expect(result[0].conflicts).toEqual([]);
    expect(original.replaces).not.toBeNull();
    expect(replacementBundle(result, [])).toBeNull();
  });
  it("keeps an existing standalone replacement", () => {
    expect(reconcilePreviews([preview("a")], [mod("a")])[0].replaces?.modId).toBe("a");
  });
  it("shows recorded versions without inventing missing bundle metadata", () => {
    expect(installedVersion([mod("a")])).toBe("Installed v1.0");
    expect(installedVersion([mod("a", "b"), mod("c", "b")])).toBe("Installed v1.0");
    expect(installedVersion([mod("a", "b"), mod("c", "b", "2.0")])).toContain("Mixed versions");
    expect(installedVersion([mod("a", null, null)])).toBe("Version not recorded");
  });
});
