// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Shell } from "../components/Shell";
import type { Dashboard, LoadOrderState } from "../types";
import { HomePage } from "./HomePage";

afterEach(cleanup);

const dashboard: Dashboard = {
  game: {
    detected: true,
    path: "/games/Star Wars Zero Company",
    steamBuildId: "24874058",
    installState: "4",
    engine: "Unreal Engine 5 (minor version unverified)",
    compatDataPath: null,
    source: "automatic"
  },
  installedMods: 2,
  enabledMods: 2,
  conflictCount: 0,
  ue4ss: {
    installed: true,
    healthy: true,
    modCount: 1,
    logFound: true,
    logPath: "/game/SWZeroCompany/Binaries/Win64/ue4ss/UE4SS.log",
    extraLoaders: [],
    vcRuntime: null,
    protonOverride: true,
    message: null
  },
  previousBuildId: null,
  dataDirectory: "/data/zcom",
  storageMode: "platform",

  existingModScanPending: false
};

function homeProps(overrides: Partial<Parameters<typeof HomePage>[0]> = {}): Parameters<typeof HomePage>[0] {
  return {
    data: dashboard, onInstall: vi.fn(), onDiagnose: vi.fn(), onLocate: vi.fn(),
    onOpenMods: vi.fn(), onOpenGame: vi.fn(), onLaunchGame: vi.fn(), onGetUe4ss: vi.fn(),
    onInstallUe4ss: vi.fn(), busy: false, launching: false, ...overrides,
  };
}

describe("truthful readiness evidence", () => {
  const noOverlaps: LoadOrderState = { entries: [], ue4ssEntries: [], activeConflicts: [], potentialConflicts: [], unapplied: true };

  it("does not call optional filename normalization a warning or unsaved change", async () => {
    render(<HomePage {...homeProps({ loadOrder: noOverlaps })} />);
    await userEvent.click(screen.getByRole("button", { name: "Load Order: ready. No recorded overlaps" }));
    expect(screen.queryByText(/reviewed ordering change/i)).toBeNull();
    expect(screen.getByText(/not a check of every asset/)).toBeTruthy();
  });

  it("keeps real recorded overlaps actionable", async () => {
    const overlap = { id: "overlap", memberIds: ["a", "b"], packageCount: 1, active: true, potential: false, winnerId: "a" };
    render(<HomePage {...homeProps({ loadOrder: { ...noOverlaps, activeConflicts: [overlap] } })} />);
    expect(screen.getByRole("button", { name: "Load Order: warning. 1 recorded overlap" })).toBeTruthy();
  });

  it("does not turn unavailable load-order data into zero conflicts", () => {
    render(<HomePage {...homeProps({ loadOrder: null, data: { ...dashboard, conflictCount: 7 } })} />);
    expect(screen.getByRole("button", { name: "Load Order: unverified. Not checked" })).toBeTruthy();
    expect(screen.queryByText(/0 active overlap/)).toBeNull();
    expect(screen.getByText("Load-order information is unavailable")).toBeTruthy();
  });

  it.each([true, false])("limits runtime readiness to files even when logFound=%s", async logFound => {
    render(<HomePage {...homeProps({ data: { ...dashboard, ue4ss: { ...dashboard.ue4ss, logFound } } })} />);
    await userEvent.click(screen.getByRole("button", { name: "UE4SS: ready. Runtime files found" }));
    expect(screen.getByText("Required runtime files are present. This does not confirm in-game loading.")).toBeTruthy();
    expect(screen.queryByText(/load is proven/)).toBeNull();
  });

  it("does not invent a default profile or verified build/catalog", () => {
    render(<HomePage {...homeProps({ profile: null, data: { ...dashboard, game: { ...dashboard.game, steamBuildId: null } }, compatibility: { status: "ready", generatedAt: "2026-09-22T08:00:00Z", catalogState: "not configured", issues: [] } })} />);
    expect(screen.queryByText("Default")).toBeNull();
    expect(screen.queryByText(/2 \/ 2 mods enabled/)).toBeNull();
    const build = screen.getByText("Build not identified").closest(".system-evidence > div")! as HTMLElement;
    const catalog = screen.getByText("Recorded catalog state: not configured").closest(".system-evidence > div")! as HTMLElement;
    expect(within(build).getByLabelText("Unverified")).toBeTruthy();
    expect(within(catalog).getByLabelText("Unverified")).toBeTruthy();
  });

  it("distinguishes saved profile selections from enabled deployment counts", () => {
    const profile = { id: "p", name: "Campaign", notes: "", requiredRuntime: null, active: true, createdAt: "", updatedAt: "", enabledMods: 0, totalMods: 2, mods: [] };
    render(<HomePage {...homeProps({ profile })} />);
    expect(screen.getByText("2 enabled mods · 2 mods in library")).toBeTruthy();
    expect(screen.getByText("0 / 2 mods selected in saved profile")).toBeTruthy();
    expect(screen.queryByText("0 / 2 mods enabled")).toBeNull();
  });
});

describe("home desktop actions", () => {
  it("offers review and dismissal when existing mods are found", async () => {
    const onReviewExisting = vi.fn();
    const onDismissExisting = vi.fn();
    render(<HomePage data={dashboard} onInstall={vi.fn()} onDiagnose={vi.fn()} onLocate={vi.fn()} onOpenMods={vi.fn()} onOpenGame={vi.fn()} onLaunchGame={vi.fn()} onGetUe4ss={vi.fn()} onInstallUe4ss={vi.fn()} busy={false} launching={false} existingModsFound={3} onReviewExisting={onReviewExisting} onDismissExisting={onDismissExisting} />);
    expect(screen.getByText("Existing mods found")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Review existing mods" }));
    await userEvent.click(screen.getByRole("button", { name: "Not now" }));
    expect(onReviewExisting).toHaveBeenCalledOnce();
    expect(onDismissExisting).toHaveBeenCalledOnce();
  });

  it("routes folder and launch buttons through explicit callbacks", async () => {
    const onOpenMods = vi.fn();
    const onOpenGame = vi.fn();
    const onLaunchGame = vi.fn();
    render(<HomePage
      data={dashboard}
      onInstall={vi.fn()}
      onDiagnose={vi.fn()}
      onLocate={vi.fn()}
      onOpenMods={onOpenMods}
      onOpenGame={onOpenGame}
      onLaunchGame={onLaunchGame}
      onGetUe4ss={vi.fn()}
      onInstallUe4ss={vi.fn()}
      busy={false}
      launching={false}
    />);

    await userEvent.click(screen.getByRole("button", { name: "Open mods folder" }));
    await userEvent.click(screen.getByRole("button", { name: "Launch game" }));
    await userEvent.click(screen.getByTitle("Open game folder"));
    expect(onOpenMods).toHaveBeenCalledOnce();
    expect(onLaunchGame).toHaveBeenCalledOnce();
    expect(onOpenGame).toHaveBeenCalledOnce();
  });

  it("disables launch while the game is unavailable", () => {
    render(<HomePage
      data={{ ...dashboard, game: { ...dashboard.game, detected: false, path: null } }}
      onInstall={vi.fn()}
      onDiagnose={vi.fn()}
      onLocate={vi.fn()}
      onOpenMods={vi.fn()}
      onOpenGame={vi.fn()}
      onLaunchGame={vi.fn()}
      onGetUe4ss={vi.fn()}
      onInstallUe4ss={vi.fn()}
      busy={false}
      launching={false}
    />);
    expect((screen.getByRole("button", { name: "Launch game" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("explains a stale saved location without hiding the old path", () => {
    render(<HomePage
      data={{ ...dashboard, game: { ...dashboard.game, detected: false, path: "D:/OldSteam/Zero Company", problemCode: "game_path_invalid", problem: "The saved game location is no longer valid. The Steam library may have moved." } }}
      onInstall={vi.fn()}
      onDiagnose={vi.fn()}
      onLocate={vi.fn()}
      onOpenMods={vi.fn()}
      onOpenGame={vi.fn()}
      onLaunchGame={vi.fn()}
      onGetUe4ss={vi.fn()}
      onInstallUe4ss={vi.fn()}
      busy={false}
      launching={false}
    />);
    expect(screen.getByRole("heading", { name: "Your saved game location is unavailable" })).toBeTruthy();
    expect(screen.getByRole("alert").querySelector("code")?.textContent).toBe("D:/OldSteam/Zero Company");
    expect(screen.getByRole("button", { name: "Locate game" })).toBeTruthy();
  });

  it("allows a configured custom launcher when Steam detection is unavailable", async () => {
    const onLaunchGame = vi.fn();
    render(<HomePage
      data={{ ...dashboard, game: { ...dashboard.game, detected: false, path: null } }}
      onInstall={vi.fn()}
      onDiagnose={vi.fn()}
      onLocate={vi.fn()}
      onOpenMods={vi.fn()}
      onOpenGame={vi.fn()}
      onLaunchGame={onLaunchGame}
      onGetUe4ss={vi.fn()}
      onInstallUe4ss={vi.fn()}
      busy={false}
      launching={false}
      canLaunch
    />);
    await userEvent.click(screen.getByRole("button", { name: "Launch game" }));
    expect(onLaunchGame).toHaveBeenCalledOnce();
  });

  it("turns holotable systems and command tabs into real actions", async () => {
    const onOpenGame = vi.fn();
    const onHealth = vi.fn();
    const onInstall = vi.fn();
    render(<HomePage
      data={dashboard}
      onInstall={onInstall}
      onDiagnose={vi.fn()}
      onLocate={vi.fn()}
      onOpenMods={vi.fn()}
      onOpenGame={onOpenGame}
      onLaunchGame={vi.fn()}
      onGetUe4ss={vi.fn()}
      onInstallUe4ss={vi.fn()}
      onHealth={onHealth}
      compatibility={{ status: "warning", generatedAt: "2026-09-22T08:00:00Z", catalogState: "verified", issues: [{ id: "rule-1", status: "warning", ruleType: "load-after", title: "Review order", detail: "A soft rule needs review.", source: "community-catalog", evidenceUrl: null, memberIds: ["a", "b"] }] }}
      busy={false}
      launching={false}
    />);

    await userEvent.click(screen.getByRole("button", { name: /Game: ready/ }));
    expect(screen.getByRole("heading", { name: "Game" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Open game folder" }));
    expect(onOpenGame).toHaveBeenCalledOnce();

    await userEvent.click(screen.getByRole("tab", { name: "operations" }));
    await userEvent.click(screen.getByRole("button", { name: "Install mod" }));
    expect(onInstall).toHaveBeenCalledOnce();
    expect(screen.getByRole("tab", { name: "operations" }).getAttribute("aria-selected")).toBe("true");
  });
});

describe("update navigation indicator", () => {
  it("shows an accessible icon beside About only when an update is available", () => {
    const { rerender } = render(<Shell page="home" onPage={vi.fn()} gameReady updateAvailable={false}><p>Page</p></Shell>);
    expect(screen.queryByLabelText("Update available")).toBeNull();
    rerender(<Shell page="home" onPage={vi.fn()} gameReady updateAvailable><p>Page</p></Shell>);
    expect(screen.getByLabelText("Update available")).toBeTruthy();
  });
});
