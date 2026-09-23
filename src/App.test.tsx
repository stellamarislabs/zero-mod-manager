// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { open } from "@tauri-apps/plugin-dialog";
import App from "./App";

const bridge = vi.hoisted(() => ({ methods: {} as Record<string, ReturnType<typeof vi.fn>>, refresh: () => {} }));
vi.mock("./services/backend", async importOriginal => ({
  ...await importOriginal<typeof import("./services/backend")>(),
  backend: new Proxy({}, { get: (_, key) => bridge.methods[String(key)] ??= vi.fn().mockResolvedValue(undefined) }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async (_name, callback) => { bridge.refresh = callback; return () => {}; }) }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), confirm: vi.fn().mockResolvedValue(true) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("./pages/HomePage", () => ({ HomePage: ({ loadOrder }: { loadOrder: unknown }) => <div>{loadOrder ? "Current order loaded" : "Order unavailable"}</div> }));

beforeEach(() => {
  bridge.methods = {};
  vi.mocked(confirm).mockClear();
  vi.mocked(open).mockReset();
  const values: Record<string, unknown> = {
    dashboard: { game: { detected: true }, ue4ss: {}, installedMods: 0, enabledMods: 0 },
    mods: [], loadOrder: { entries: [], ue4ssEntries: [], activeConflicts: [], potentialConflicts: [], unapplied: false },
    settings: { gamePath: null, customExecutablePath: null, reducedMotion: false },
    links: {}, managedLibrary: { path: "/library" }, sevenZipStatus: { found: false }, profiles: [], activeProfile: null,
    snapshots: [], compatibility: null, launchPreflight: { status: "ready" }, activity: [], launchSessions: [], activeGuidedIsolation: null,
    supportBundlePreview: null, legacyImportStatus: { available: false },
  };
  for (const [key, value] of Object.entries(values)) bridge.methods[key] = vi.fn().mockResolvedValue(value);
});

it("offers a named override after rollback, then removes the record only after consent", async () => {
  const mod = {
    id: "helmet", bundleId: null, name: "Hub Helmet Hide", version: "1.0.1", modType: "ue4ss",
    enabled: true, hidden: false, installedAt: "2026-09-22", files: [], conflictCount: 0,
    potentialConflictCount: 0, packageCount: 0, nexusUrl: null, fomod: false
  } as const;
  let removed = false;
  bridge.methods.mods = vi.fn(async () => removed ? [] : [mod]);
  bridge.methods.uninstall = vi.fn()
    .mockRejectedValueOnce("No package changes were kept. Previous files and library were restored. A managed file changed outside Zero Mod Manager: C:\\Games\\ue4ss\\Mods\\HelmetHide\\dlls\\main.dll")
    .mockImplementationOnce(async () => { removed = true; });
  render(<App />);
  await screen.findByText("Current order loaded");
  await userEvent.click(screen.getByRole("button", { name: "Library" }));
  await userEvent.click(screen.getByRole("button", { name: "Uninstall Hub Helmet Hide" }));
  await waitFor(() => expect(bridge.methods.uninstall).toHaveBeenCalledTimes(2));
  expect(bridge.methods.uninstall).toHaveBeenNthCalledWith(1, "helmet");
  expect(bridge.methods.uninstall).toHaveBeenNthCalledWith(2, "helmet", true);
  expect(vi.mocked(confirm)).toHaveBeenCalledWith(expect.stringContaining("dlls\\main.dll"), expect.objectContaining({ okLabel: "Delete changed files" }));
  await waitFor(() => expect(screen.queryByRole("button", { name: "Uninstall Hub Helmet Hide" })).toBeNull());
});

it("retries a single mod replacement only after showing the changed DLL", async () => {
  const previous = {
    id: "old", bundleId: null, name: "Helmet Hide", version: "1.0.1", modType: "ue4ss",
    enabled: true, hidden: false, installedAt: "2026-09-22", files: [], conflictCount: 0,
    potentialConflictCount: 0, packageCount: 0, nexusUrl: null, fomod: false
  } as const;
  const candidate = {
    stagingId: "staged", sourcePath: "C:/Downloads/HelmetHide-1.0.2.zip", name: "Helmet Hide", version: "1.0.2",
    author: null, description: null, modType: "ue4ss", files: ["HelmetHide/dlls/main.dll"], warnings: [],
    valid: true, verification: "not-required", verificationDetails: null, packageCount: 0, packageNames: [],
    compatibility: "unknown", compatibilityMessage: "Unknown", testedBuilds: [], conflicts: [],
    replaces: { modId: "old", name: "Helmet Hide", version: "1.0.1", reason: "Same UE4SS folder" },
    recommendedPriority: null, loadOrderSupported: false, loadOrderSupportReason: null, optionLabel: null
  };
  bridge.methods.mods = vi.fn().mockResolvedValue([previous]);
  bridge.methods.inspect = vi.fn().mockResolvedValue({ previews: [candidate], installer: null, package: null });
  bridge.methods.install = vi.fn()
    .mockRejectedValueOnce("No package changes were kept. Previous files and library were restored. A managed file changed outside Zero Mod Manager: C:\\Games\\ue4ss\\Mods\\HelmetHide\\dlls\\main.dll")
    .mockResolvedValueOnce({ ...previous, id: "new", version: "1.0.2" });
  vi.mocked(open).mockResolvedValueOnce("C:/Downloads/HelmetHide-1.0.2.zip");
  render(<App />);
  await screen.findByText("Current order loaded");
  await userEvent.click(screen.getByRole("button", { name: "Install" }));
  await userEvent.click(screen.getByRole("button", { name: "Choose archive or file" }));
  await userEvent.click(await screen.findByRole("button", { name: "Replace installed version" }));
  await waitFor(() => expect(bridge.methods.install).toHaveBeenCalledTimes(2));
  expect(bridge.methods.install).toHaveBeenNthCalledWith(1, "staged", undefined, "old", false);
  expect(bridge.methods.install).toHaveBeenNthCalledWith(2, "staged", undefined, "old", true);
  expect(vi.mocked(confirm)).toHaveBeenCalledWith(expect.stringContaining("dlls\\main.dll"), expect.objectContaining({ okLabel: "Replace changed files" }));
});
afterEach(cleanup);

it("pauses instead of presenting an unreadable library as empty", async () => {
  bridge.methods.mods.mockRejectedValue(new Error("library unavailable"));
  render(<App />);
  expect(await screen.findByText("App data could not be loaded")).toBeTruthy();
  expect(screen.queryByText("Current order loaded")).toBeNull();
  expect(screen.queryByRole("button", { name: "Launch modded" })).toBeNull();
  bridge.methods.mods.mockResolvedValue([]);
  await userEvent.click(screen.getByRole("button", { name: "Retry loading" }));
  expect(await screen.findByText("Current order loaded")).toBeTruthy();
});

it("invalidates previous ordering evidence after a failed refresh", async () => {
  render(<App />);
  expect(await screen.findByText("Current order loaded")).toBeTruthy();
  bridge.methods.loadOrder.mockRejectedValue(new Error("order unavailable"));
  await act(async () => bridge.refresh());
  expect(await screen.findByText("Order unavailable")).toBeTruthy();
  expect(screen.queryByText("Current order loaded")).toBeNull();
});

it("does not leave launch enabled after preflight data is lost", async () => {
  render(<App />);
  await screen.findByText("Current order loaded");
  expect(screen.getByRole("button", { name: "Launch modded" }).hasAttribute("disabled")).toBe(false);
  bridge.methods.launchPreflight.mockRejectedValue(new Error("preflight unavailable"));
  await act(async () => bridge.refresh());
  expect(screen.getByRole("button", { name: "Launch modded" }).hasAttribute("disabled")).toBe(true);
});
