// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { IsolationSession, ProfileDetail } from "../types";
import { HealthPage } from "./HealthPage";
import { ProfilesPage } from "./ProfilesPage";

afterEach(cleanup);

const profile: ProfileDetail = {
  id: "profile", name: "Campaign", notes: "Act II", requiredRuntime: null,
  createdAt: "2026-09-21T00:00:00Z", updatedAt: "2026-09-21T00:00:00Z",
  active: true, enabledMods: 1, totalMods: 1,
  mods: [{ modId: "core", name: "Core", modType: "ue4ss", enabled: true, loadPriority: 1, fomodAnswers: null }],
};

describe("operational pages", () => {
  it("previews a profile switch and exposes checkpoint restoration", async () => {
    const onPreview = vi.fn();
    const onRestore = vi.fn();
    render(<ProfilesPage profiles={[profile]} selected={{ ...profile, active: false }} preview={null} snapshots={[{ id: "snap", profileId: "profile", label: "Known good", kind: "manual", createdAt: "2026-09-21T00:00:00Z", lastKnownGood: true }]} busy={false} onSelect={vi.fn()} onCreate={vi.fn()} onSave={vi.fn()} onDelete={vi.fn()} onSetMod={vi.fn()} onPreview={onPreview} onActivate={vi.fn()} onExport={vi.fn()} onImport={vi.fn()} onSnapshot={vi.fn()} onRestoreSnapshot={onRestore} />);
    await userEvent.click(screen.getByRole("button", { name: "Review switch" }));
    await userEvent.click(screen.getByRole("button", { name: "Restore as profile" }));
    expect(onPreview).toHaveBeenCalledWith("profile");
    expect(onRestore).toHaveBeenCalledOnce();
    expect(screen.getByText("Last known good")).toBeTruthy();
  });

  it("runs and records a dependency-safe guided isolation step", async () => {
    const isolation: IsolationSession = {
      id: "iso", profileId: "profile", phase: "bisect", status: "active",
      candidateGroups: [["core", "framework"], ["visuals"]], currentModIds: ["core", "framework"],
      observations: [], suspectedModIds: [], instruction: "Launch the dependency-safe subset.", createdAt: "2026-09-21T00:00:00Z",
    };
    const onRun = vi.fn();
    const onRecord = vi.fn();
    render(<HealthPage diagnostics={null} preflight={null} compatibility={null} ue4ss={null} configs={[]} configHistory={[]} activity={[]} sessions={[]} isolation={isolation} supportPreview={null} configPreview={null} loading={false} onRefresh={vi.fn()} onCopyDiagnostics={vi.fn()} onOpenLogs={vi.fn()} onOpenUe4ssLog={vi.fn()} onPreviewConfig={vi.fn()} onApplyConfig={vi.fn()} onRollbackConfig={vi.fn()} onCreateSupportBundle={vi.fn()} onCompleteSession={vi.fn()} onStartIsolation={vi.fn()} onRunIsolationStep={onRun} onRecordIsolation={onRecord} onCancelIsolation={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Guided Isolation" }));
    await userEvent.click(screen.getByRole("button", { name: "Launch this step" }));
    await userEvent.click(screen.getByRole("button", { name: "Crashed" }));
    expect(onRun).toHaveBeenCalledWith(isolation);
    expect(onRecord).toHaveBeenCalledWith(isolation, "crashed");
  });
});
