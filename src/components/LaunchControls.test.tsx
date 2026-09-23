// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LaunchControls } from "./LaunchControls";
import { Shell, type Page } from "./Shell";

afterEach(cleanup);
describe("persistent application controls", () => {
  it("requires explicit confirmation before troubleshooting", async () => {
    const onLaunchMode = vi.fn();
    render(<LaunchControls status="ready" canLaunch launching={false} onHealth={vi.fn()} onLaunchGame={vi.fn()} onLaunchMode={onLaunchMode} onProfiles={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Troubleshoot" }));
    expect(onLaunchMode).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onLaunchMode).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Troubleshoot" }));
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(onLaunchMode).toHaveBeenCalledExactlyOnceWith("troubleshoot");
  });
  it("keeps the same sidebar and toolbar nodes across navigation", () => {
    const toolbar = <LaunchControls status="unverified" canLaunch launching={false} onHealth={vi.fn()} onLaunchGame={vi.fn()} onLaunchMode={vi.fn()} onProfiles={vi.fn()} />;
    const app = (page: Page) => <Shell page={page} onPage={vi.fn()} gameReady updateAvailable={false} toolbar={toolbar}><h1>{page}</h1></Shell>;
    const { rerender, container } = render(app("home"));
    const navigation = screen.getByRole("navigation");
    const controls = screen.getByRole("region", { name: "Launch and readiness controls" });
    for (const page of ["mods", "install", "profiles", "diagnostics", "settings", "about", "home"] as Page[]) {
      rerender(app(page));
      expect(screen.getByRole("navigation")).toBe(navigation);
      expect(screen.getByRole("region", { name: "Launch and readiness controls" })).toBe(controls);
      expect(container.querySelector(".shell")?.className).toBe("shell");
      expect(screen.getAllByRole("button", { name: "Launch modded" })).toHaveLength(1);
    }
  });
  it("preserves launch mode callbacks and blocked/launching guards", async () => {
    const onLaunchMode = vi.fn();
    const props = { status: "blocked" as const, canLaunch: true, launching: false, onHealth: vi.fn(), onLaunchGame: vi.fn(), onLaunchMode, onProfiles: vi.fn() };
    const { rerender } = render(<LaunchControls {...props} />);
    expect((screen.getByRole("button", { name: "Launch modded" }) as HTMLButtonElement).disabled).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "Launch vanilla" }));
    expect(onLaunchMode).not.toHaveBeenCalled();
    expect((screen.getByRole("button", { name: "Troubleshoot" }) as HTMLButtonElement).disabled).toBe(true);
    rerender(<LaunchControls {...props} status="ready" />);
    await userEvent.click(screen.getByRole("button", { name: "Launch vanilla" }));
    expect(onLaunchMode).toHaveBeenCalledWith("vanilla");
    await userEvent.click(screen.getByRole("button", { name: "Launch modded" }));
    expect(onLaunchMode).toHaveBeenCalledWith("modded");
    rerender(<LaunchControls {...props} status="ready" launching />);
    expect((screen.getByRole("button", { name: "Launch vanilla" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Troubleshoot" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
