// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FactoryResetPanel } from "./FactoryResetPanel";

afterEach(cleanup);

async function openReset() {
  await userEvent.click(screen.getByRole("button", { name: "Reset app data…" }));
  return screen.getByRole("dialog", { name: "Reset Zero Mod Manager?" });
}

async function acknowledge() {
  await userEvent.click(screen.getByRole("checkbox", { name: /manager will forget/ }));
  await userEvent.type(screen.getByLabelText("Type RESET to confirm"), "RESET");
}

describe("factory reset", () => {
  it("explains preservation, requires both safeguards and focuses Cancel first", async () => {
    const onReset = vi.fn().mockResolvedValue(undefined);
    render(<FactoryResetPanel onReset={onReset} />);
    const dialog = await openReset();
    expect(onReset).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(within(dialog).getByText(/installed game mods and saves stay untouched/)).toBeTruthy();
    expect(within(dialog).getByText(/local recovery backup is kept/)).toBeTruthy();
    expect(within(dialog).getByText(/Original-file backups/)).toBeTruthy();
    const confirm = within(dialog).getByRole("button", { name: "Reset app and close" }) as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    await userEvent.type(screen.getByLabelText("Type RESET to confirm"), "RESET");
    expect(confirm.disabled).toBe(true);
    await userEvent.click(screen.getByRole("checkbox", { name: /manager will forget/ }));
    expect(confirm.disabled).toBe(false);
    await userEvent.click(confirm);
    expect(onReset).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Preparing reset…" }).hasAttribute("disabled")).toBe(true);
  });

  it("cancels without resetting and clears previous confirmation when reopened", async () => {
    const onReset = vi.fn().mockResolvedValue(undefined);
    render(<FactoryResetPanel onReset={onReset} />);
    await openReset();
    await acknowledge();
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onReset).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
    await openReset();
    expect((screen.getByLabelText("Type RESET to confirm") as HTMLInputElement).value).toBe("");
    expect((screen.getByRole("checkbox", { name: /manager will forget/ }) as HTMLInputElement).checked).toBe(false);
  });

  it("does not accept an incomplete or differently cased confirmation", async () => {
    const onReset = vi.fn().mockResolvedValue(undefined);
    render(<FactoryResetPanel onReset={onReset} />);
    await openReset();
    await userEvent.click(screen.getByRole("checkbox", { name: /manager will forget/ }));
    await userEvent.type(screen.getByLabelText("Type RESET to confirm"), "reset");
    await userEvent.click(screen.getByRole("button", { name: "Reset app and close" }));
    expect(onReset).not.toHaveBeenCalled();
  });

  it("preserves the explanation and input after failure and permits retry or cancel", async () => {
    const onReset = vi.fn().mockRejectedValueOnce(new Error("Finish the pending operation first.")).mockResolvedValue(undefined);
    render(<FactoryResetPanel onReset={onReset} />);
    await openReset();
    await acknowledge();
    await userEvent.click(screen.getByRole("button", { name: "Reset app and close" }));
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText("Finish the pending operation first.")).toBeTruthy();
    expect((screen.getByLabelText("Type RESET to confirm") as HTMLInputElement).value).toBe("RESET");
    expect((screen.getByRole("button", { name: "Cancel" }) as HTMLButtonElement).disabled).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Reset app and close" }));
    await waitFor(() => expect(onReset).toHaveBeenCalledTimes(2));
  });

  it("disables the action while a reset or library move is active", async () => {
    const onReset = vi.fn();
    const { rerender } = render(<FactoryResetPanel onReset={onReset} resetting />);
    await userEvent.click(screen.getByRole("button", { name: "Reset app data…" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    rerender(<FactoryResetPanel onReset={onReset} disabled />);
    expect((screen.getByRole("button", { name: "Reset app data…" }) as HTMLButtonElement).disabled).toBe(true);
    expect(onReset).not.toHaveBeenCalled();
  });

  it("discloses that a custom external library is retained", async () => {
    render(<FactoryResetPanel onReset={vi.fn()} retainedLibraryPath={"D:\\My mod library"} />);
    await openReset();
    expect(screen.getByText("D:\\My mod library")).toBeTruthy();
    expect(screen.getByText(/will remain. App records and settings still reset/)).toBeTruthy();
  });
});
