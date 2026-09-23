// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ProfilesPage } from "./ProfilesPage";

afterEach(cleanup);

function props(): Parameters<typeof ProfilesPage>[0] {
  const selected = { id: "p", name: "Test", notes: "", requiredRuntime: null, createdAt: "", updatedAt: "", active: false, enabledMods: 1, totalMods: 1,
    mods: [{ modId: "m", name: "Armor", modType: "pak" as const, enabled: true, loadPriority: 2, fomodAnswers: null }] };
  return { profiles: [selected], selected, snapshots: [], busy: false,
    preview: { profileId: "p", profileName: "Test", blocked: false, reasons: [], changes: [{ modId: "m", name: "Armor", fromEnabled: true, toEnabled: true, fromPriority: 1, toPriority: 2 }] },
    onSelect: vi.fn(), onCreate: vi.fn(), onSave: vi.fn(), onDelete: vi.fn(), onSetMod: vi.fn(), onPreview: vi.fn(), onActivate: vi.fn(), onExport: vi.fn(), onImport: vi.fn(), onSnapshot: vi.fn(), onRestoreSnapshot: vi.fn() };
}

describe("accurate profile review", () => {
  it("describes saved selections without claiming they are already deployed", () => {
    render(<ProfilesPage {...props()} />);
    expect(screen.getByText("1/1 mods selected")).toBeTruthy();
    expect(screen.getByText("Saved selection")).toBeTruthy();
    expect(screen.getByText("Selections are saved to this profile. Review and apply to change installed mods.")).toBeTruthy();
    expect(screen.queryByText("1/1 mods on")).toBeNull();
  });

  it("allows an active profile's saved changes to be reviewed without automatic deployment", async () => {
    const handlers = props();
    const active = { ...handlers.selected!, active: true };
    render(<ProfilesPage {...handlers} selected={active} profiles={[active]} preview={null} />);
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Armor for Test" }));
    expect(handlers.onSetMod).toHaveBeenCalledWith("m", false, 2);
    expect(handlers.onActivate).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Review saved changes" }));
    expect(handlers.onPreview).toHaveBeenCalledExactlyOnceWith("p");
    expect(handlers.onActivate).not.toHaveBeenCalled();
  });

  it("shows priority-only changes instead of a misleading Enabled to Enabled change", () => {
    render(<ProfilesPage {...props()} />);
    expect(screen.getByText("Priority 1 → 2")).toBeTruthy();
    expect(screen.queryByText("Enabled → Enabled")).toBeNull();
    expect(screen.getByText("Mod / component")).toBeTruthy();
  });

  it("does not display an unsaved toggle as a confirmed saved state", async () => {
    const handlers = props();
    render(<ProfilesPage {...handlers} />);
    const enabled = screen.getByRole("checkbox") as HTMLInputElement;
    await userEvent.click(enabled);
    expect(handlers.onSetMod).toHaveBeenCalledWith("m", false, 2);
    expect(enabled.checked).toBe(true);
  });
});
