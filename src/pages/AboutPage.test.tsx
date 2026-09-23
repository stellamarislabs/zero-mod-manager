// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AboutPage } from "./AboutPage";

afterEach(cleanup);

describe("About update status", () => {
  it("does not present an older successful check as current after refresh fails", () => {
    render(<AboutPage projectUrl="https://github.com/stellamarislabs/zero-mod-manager" nexusUrl="" onOpenLink={vi.fn()}
      update={{ currentVersion: "0.7.0-rc.4", latestVersion: "0.7.0-rc.2", releaseUrl: "", updateAvailable: false }}
      checking={false} error="Offline" onCheckUpdates={vi.fn()} />);
    expect(screen.queryByText(/No newer release found|up to date/)).toBeNull();
    expect(screen.getByText(/Updates could not be checked/)).toBeTruthy();
  });
  it("only claims that no newer release was found, not that a local candidate is published", () => {
    render(<AboutPage projectUrl="https://github.com/stellamarislabs/zero-mod-manager" nexusUrl="" onOpenLink={vi.fn()}
      update={{ currentVersion: "0.7.0-rc.4", latestVersion: "0.7.0-rc.2", releaseUrl: "", updateAvailable: false }}
      checking={false} error={null} onCheckUpdates={vi.fn()} />);
    expect(screen.getByText(/No newer release found. Latest checked GitHub release: v0.7.0-rc.2/)).toBeTruthy();
    expect(screen.queryByText(/You’re up to date/)).toBeNull();
  });
  it("does not show an unchecked or in-progress result as success", () => {
    const base = { projectUrl: "https://github.com/stellamarislabs/zero-mod-manager", nexusUrl: "", onOpenLink: vi.fn(), error: null, onCheckUpdates: vi.fn() };
    const { rerender } = render(<AboutPage {...base} update={null} checking={false} />);
    expect(screen.getByText("Update status not checked.")).toBeTruthy();
    rerender(<AboutPage {...base} checking update={{ currentVersion: "1.0.0", latestVersion: "1.0.0", releaseUrl: "", updateAvailable: false }} />);
    expect(screen.queryByText(/No newer release found/)).toBeNull();
  });
  it("does not call an unpublished release up to date or an error", () => {
    render(<AboutPage projectUrl="https://github.com/stellamarislabs/zero-mod-manager" nexusUrl="" onOpenLink={vi.fn()} update={{currentVersion:"0.7.0-rc.2",latestVersion:"0.7.0-rc.2",releaseUrl:"",updateAvailable:false,releaseAvailable:false}} checking={false} error={null} onCheckUpdates={vi.fn()} />);
    expect(screen.getByText("No public release is available yet.")).toBeTruthy();
    expect(screen.queryByText(/You’re up to date/)).toBeNull();
  });
  it("shows the startup result and opens the published release", async () => {
    const onOpenLink = vi.fn();
    render(<AboutPage
      projectUrl="https://github.com/arctco/zcom-mod-manager"
      nexusUrl="https://www.nexusmods.com/starwarszerocompany/mods/29"
      onOpenLink={onOpenLink}
      update={{
        currentVersion: "0.2.0",
        latestVersion: "0.2.1",
        releaseUrl: "https://github.com/arctco/zcom-mod-manager/releases/tag/v0.2.1",
        updateAvailable: true
      }}
      checking={false}
      error={null}
      onCheckUpdates={vi.fn()}
    />);

    expect(screen.getByText("Version 0.2.1 is available.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /open release page/i }));
    expect(onOpenLink).toHaveBeenCalledWith("https://github.com/arctco/zcom-mod-manager/releases/tag/v0.2.1");
  });

  it("lets an offline user retry without hiding the error", async () => {
    const onCheckUpdates = vi.fn();
    render(<AboutPage
      projectUrl="https://github.com/arctco/zcom-mod-manager"
      nexusUrl="https://www.nexusmods.com/starwarszerocompany/mods/29"
      onOpenLink={vi.fn()}
      update={null}
      checking={false}
      error="Network unavailable"
      onCheckUpdates={onCheckUpdates}
    />);

    expect(screen.getByText(/Updates could not be checked/)).toBeTruthy();
    await userEvent.click(screen.getByText("Technical details"));
    expect(screen.getByText("Network unavailable")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Check again" }));
    expect(onCheckUpdates).toHaveBeenCalledOnce();
  });
});

describe("where an update can be taken from", () => {
  it("offers the Nexus Mods page beside the GitHub release", async () => {
    const onOpenLink = vi.fn();
    render(<AboutPage
      projectUrl="https://github.com/arctco/zcom-mod-manager"
      nexusUrl="https://www.nexusmods.com/starwarszerocompany/mods/29"
      onOpenLink={onOpenLink}
      update={{
        currentVersion: "0.4.1",
        latestVersion: "0.5.0",
        releaseUrl: "https://github.com/arctco/zcom-mod-manager/releases/tag/v0.5.0",
        updateAvailable: true
      }}
      checking={false}
      error={null}
      onCheckUpdates={vi.fn()}
    />);

    await userEvent.click(screen.getByRole("button", { name: /view nexus mods page/i }));
    expect(onOpenLink).toHaveBeenCalledWith("https://www.nexusmods.com/starwarszerocompany/mods/29");
  });

  it("keeps the page reachable when no update is waiting", async () => {
    const onOpenLink = vi.fn();
    render(<AboutPage
      projectUrl="https://github.com/arctco/zcom-mod-manager"
      nexusUrl="https://www.nexusmods.com/starwarszerocompany/mods/29"
      onOpenLink={onOpenLink}
      update={{ currentVersion: "0.4.1", latestVersion: "0.4.1", releaseUrl: "", updateAvailable: false }}
      checking={false}
      error={null}
      onCheckUpdates={vi.fn()}
    />);

    await userEvent.click(screen.getByRole("button", { name: /nexus mods page/i }));
    expect(onOpenLink).toHaveBeenCalledWith("https://www.nexusmods.com/starwarszerocompany/mods/29");
  });
});
