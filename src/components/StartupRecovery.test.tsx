// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StartupRecovery } from "./StartupRecovery";

afterEach(cleanup);

describe("startup recovery", () => {
  it("keeps a bootstrap failure visible and actionable", async () => {
    const retry = vi.fn();
    const logs = vi.fn();
    render(<StartupRecovery message="database is locked" retrying={false} onRetry={retry} onOpenLogs={logs} />);

    expect(screen.getByRole("alert")).toBeDefined();
    expect(screen.getByText("database is locked")).toBeDefined();
    await userEvent.click(screen.getByRole("button", { name: "Retry loading" }));
    await userEvent.click(screen.getByRole("button", { name: "Open logs" }));
    expect(retry).toHaveBeenCalledOnce();
    expect(logs).toHaveBeenCalledOnce();
  });

  it("prevents duplicate retries while startup is running", () => {
    render(<StartupRecovery message="still checking" retrying onRetry={() => {}} onOpenLogs={() => {}} />);
    expect(screen.getByRole("button", { name: "Retrying…" }).hasAttribute("disabled")).toBe(true);
  });
});
