// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LibraryGrouping } from "./LibraryGrouping";
import { groupMods } from "../utils/modGroups";
import type { ModSummary } from "../types";
afterEach(cleanup);
const mod = (id: string, extra: Partial<ModSummary> = {}): ModSummary => ({ id, name: id, modType: "pak", bundleId: null, files: [], enabled: true, ...extra } as ModSummary);
it("retains real bundle identity and component names", () => {
  const grouped = groupMods([mod("A", { bundleId: "b", bundleName: "Paint" }), mod("B", { bundleId: "b", bundleName: "Paint" })]);
  expect(grouped[0].name).toBe("Paint");
  expect(grouped[0].members.map(item => item.name)).toEqual(["A", "B"]);
});
it("never groups unrelated entries by their display names", () => {
  const items = Array.from({ length: 5 }, (_, index) => mod(`armor-${index}`, { name: "Armor" }));
  expect(groupMods(items)).toHaveLength(5);
});
it("offers only reviewed separation, not arbitrary grouping", async () => {
  const onUngroup = vi.fn().mockResolvedValue(undefined);
  render(<LibraryGrouping mods={[mod("Alpha", { bundleId: "b", bundleName: "Wrong merge" }), mod("Beta", { bundleId: "b" })]} busy={false} onUngroup={onUngroup} onRestore={vi.fn()} />);
  await userEvent.click(screen.getByRole("button", { name: "Organize mods" }));
  expect(screen.queryByRole("button", { name: /group as one mod/i })).toBeNull();
  await userEvent.click(screen.getByRole("button", { name: "Separate mods" }));
  expect(onUngroup).not.toHaveBeenCalled();
  const dialog = screen.getByRole("dialog");
  expect(within(dialog).getByText("Alpha")).toBeTruthy();
  expect(within(dialog).getByText("Beta")).toBeTruthy();
  await userEvent.click(within(dialog).getByRole("button", { name: "Separate mods" }));
  expect(onUngroup).toHaveBeenCalledWith("Alpha");
});
it("explains legacy recovery and allows cancellation without mutation", async () => {
  const onRestore = vi.fn();
  const legacy = mod("Legacy", { files: [{ name: "One.pak" }, { name: "Two.pak" }] as ModSummary["files"] });
  render(<LibraryGrouping mods={[legacy]} busy={false} onUngroup={vi.fn()} onRestore={onRestore} />);
  await userEvent.click(screen.getByRole("button", { name: "Organize mods" }));
  await userEvent.click(screen.getByText("Recover older merged records"));
  await userEvent.click(screen.getByRole("button", { name: "Recover separate mods" }));
  expect(screen.getByText(/Previously exported profile files/)).toBeTruthy();
  expect(screen.getByText(/separate Library row for each complete file set/)).toBeTruthy();
  await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(onRestore).not.toHaveBeenCalled();
});
