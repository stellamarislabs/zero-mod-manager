// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Shell } from "../components/Shell";
import type { Dashboard } from "../types";
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
  retoc: { found: true, path: "/bin/retoc", version: "retoc 0.1.5" },
  existingModScanPending: false
};

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
    expect(screen.getByRole("heading", { name: "Your saved game location moved" })).toBeTruthy();
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
