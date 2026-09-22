// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { AppSettings } from "../types";
import { SettingsPage } from "./SettingsPage";

afterEach(cleanup);

const settings: AppSettings = {
  gamePath: null,
  customExecutablePath: "C:\\Games\\ZeroCompany.exe",
  retocPath: null,
  sevenZipPath: null,
  logLevel: "normal",
  advancedPackageNames: false,
  reducedMotion: false,
};

function props(overrides: Partial<Parameters<typeof SettingsPage>[0]> = {}): Parameters<typeof SettingsPage>[0] {
  return {
    settings,
    retoc: { found: true, path: "/bin/retoc", version: "retoc 0.1.5" },
    sevenZip: { found: true, path: "C:\\Program Files\\7-Zip\\7z.exe", version: "7-Zip 24.09" },
    managedLibrary: { path: "C:\\ZCOM Mods", defaultPath: "C:\\Users\\Arc\\AppData\\Local\\ZCOM Mods", isDefault: false },
    movingLibrary: false,
    onChange: vi.fn(),
    onSave: vi.fn(),
    onPickGame: vi.fn(),
    onPickExecutable: vi.fn(),
    onPickRetoc: vi.fn(),
    onPickSevenZip: vi.fn(),
    onMoveLibrary: vi.fn(),
    onUseDefaultLibrary: vi.fn(),
    onOpenLibrary: vi.fn(),
    onOpenLogs: vi.fn(),
    onOpenData: vi.fn(),
    links: { ue4ssDownload: "", nexusGame: "", nexusManager: "", project: "" },
    onOpenLink: vi.fn(),
    ...overrides
  };
}

describe("custom game launcher", () => {
  it("shows, browses, and clears a custom executable", async () => {
    const onPickExecutable = vi.fn();
    const onChange = vi.fn();
    render(<SettingsPage {...props({ onPickExecutable, onChange })} />);

    const input = screen.getByLabelText("Game launch executable or launcher") as HTMLInputElement;
    expect(input.value).toBe("C:\\Games\\ZeroCompany.exe");
    const label = input.closest("label");
    expect(label).not.toBeNull();
    await userEvent.click(within(label!).getByRole("button", { name: "Browse" }));
    await userEvent.click(within(label!).getByRole("button", { name: "Use Steam" }));

    expect(onPickExecutable).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith({ ...settings, customExecutablePath: null });
  });

  it("describes Steam as the default when no custom path is set", () => {
    render(<SettingsPage {...props({ settings: { ...settings, customExecutablePath: null } })} />);
    expect((screen.getByLabelText("Game launch executable or launcher") as HTMLInputElement).value).toBe("Steam default");
    expect(screen.queryByRole("button", { name: "Use Steam" })).toBeNull();
  });
});

describe("managed mod library", () => {
  it("shows the active path and offers a verified move", async () => {
    const onMoveLibrary = vi.fn();
    const onUseDefaultLibrary = vi.fn();
    render(<SettingsPage {...props({ onMoveLibrary, onUseDefaultLibrary })} />);

    expect((screen.getByLabelText("Library folder") as HTMLInputElement).value).toBe("C:\\ZCOM Mods");
    await userEvent.click(screen.getByRole("button", { name: "Move…" }));
    await userEvent.click(screen.getByRole("button", { name: "Use Local AppData" }));
    expect(onMoveLibrary).toHaveBeenCalledOnce();
    expect(onUseDefaultLibrary).toHaveBeenCalledOnce();
  });
});

describe("local tools", () => {
  it("has no account or API controls and describes retoc as optional", () => {
    render(<SettingsPage {...props({ retoc: { found: false, path: null, version: null } })} />);
    expect(screen.queryByText(/API key/i)).toBeNull();
    expect(screen.queryByRole("checkbox", { name: /Check installed mods for updates/ })).toBeNull();
    expect(screen.getByRole("heading", { name: "Container verification (optional)" })).toBeDefined();
  });
});
