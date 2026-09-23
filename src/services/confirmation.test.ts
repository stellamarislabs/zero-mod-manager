import { describe, expect, it, vi } from "vitest";
import { confirm } from "@tauri-apps/plugin-dialog";
import { requestConfirmation } from "./confirmation";
import capabilities from "../../src-tauri/capabilities/default.json";
vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn() }));

describe("native confirmation", () => {
  it("waits for explicit consent", async () => {
    let answer!: (result: boolean) => void;
    vi.mocked(confirm).mockImplementationOnce(() => new Promise(resolve => { answer = resolve; }));
    const mutate = vi.fn();
    const operation = requestConfirmation("Replace?", vi.fn()).then(yes => { if (yes) mutate(); });
    await Promise.resolve();
    expect(mutate).not.toHaveBeenCalled();
    answer(true);
    await operation;
    expect(mutate).toHaveBeenCalledOnce();
  });
  it("cancellation and unavailable dialogs fail closed", async () => {
    vi.mocked(confirm).mockResolvedValueOnce(false).mockRejectedValueOnce(new Error("ACL denied"));
    const report = vi.fn();
    expect(await requestConfirmation("Delete?", report)).toBe(false);
    expect(await requestConfirmation("Delete?", report)).toBe(false);
    expect(report).toHaveBeenCalledOnce();
  });
  it("names the destructive decision and never proceeds after a rejected dialog", async () => {
    vi.mocked(confirm).mockResolvedValueOnce(true).mockRejectedValueOnce(new Error("unavailable"));
    expect(await requestConfirmation("The changed DLL will be replaced.", vi.fn(), "Replace changed file")).toBe(true);
    expect(vi.mocked(confirm)).toHaveBeenCalledWith("The changed DLL will be replaced.", expect.objectContaining({ okLabel: "Replace changed file" }));
    expect(await requestConfirmation("Delete changed file?", vi.fn(), "Delete changed file")).toBe(false);
  });
  it("permits only the native dialog operations actually used", () => {
    expect(capabilities.permissions).toContain("dialog:allow-confirm");
    expect(capabilities.permissions).toContain("dialog:allow-save");
    expect(capabilities.permissions).not.toContain("dialog:default");
  });
});
