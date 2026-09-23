// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { HealthPage } from "./HealthPage";
import { DiagnosticsPage } from "./DiagnosticsPage";

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

function props(overrides: Partial<Parameters<typeof HealthPage>[0]> = {}): Parameters<typeof HealthPage>[0] {
  return {
    diagnostics: null, preflight: null, compatibility: null, ue4ss: null,
    configs: [], configHistory: [], activity: [], sessions: [], supportPreview: null,
    isolation: null, configPreview: null, loading: false,
    onRefresh: vi.fn(), onCopyDiagnostics: vi.fn(), onOpenLogs: vi.fn(), onOpenUe4ssLog: vi.fn(),
    onPreviewConfig: vi.fn(), onApplyConfig: vi.fn(), onRollbackConfig: vi.fn(), onCreateSupportBundle: vi.fn(),
    onCompleteSession: vi.fn(), onStartIsolation: vi.fn(), onRunIsolationStep: vi.fn(), onRecordIsolation: vi.fn(), onCancelIsolation: vi.fn(),
    ...overrides
  };
}

describe("evidence-based health", () => {
  it("does not turn missing reports into no problems", () => {
    render(<HealthPage {...props()} />);
    expect(screen.getAllByText("Not checked")).toHaveLength(3);
    expect(screen.getByText("Run checks to see reported issues")).toBeTruthy();
    expect(screen.queryByText(/0 findings|None detected|None found/)).toBeNull();
  });

  it("distinguishes present runtime files and historical logs from a loaded runtime", () => {
    render(<HealthPage {...props({ ue4ss: { installed: true, healthy: true, modCount: 2, logFound: true, logPath: "log", extraLoaders: [], vcRuntime: true, protonOverride: null, message: null } })} />);
    expect(screen.getByText("Present")).toBeTruthy();
    expect(screen.getByText("Found (may be from an earlier session)")).toBeTruthy();
    expect(screen.queryByText("Observed")).toBeNull();
  });

  it("cannot save or assert content for an unavailable support preview", async () => {
    render(<HealthPage {...props()} />);
    await userEvent.click(screen.getByRole("button", { name: "Support" }));
    expect(screen.queryByText("Save data included: No")).toBeNull();
    expect((screen.getByRole("button", { name: "Save support bundle" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Copy diagnostics only" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("keeps an unknown session result reportable and identifies user-reported outcomes", async () => {
    const base = { mode: "modded" as const, profileId: "p", launcher: "Steam", gameBuild: null, startedAt: "2026-09-22T10:00:00Z", endedAt: null, logEvidence: null };
    const handlers = props({ sessions: [{ ...base, id: "a", outcome: "unknown" }, { ...base, id: "b", outcome: "worked" }] });
    render(<HealthPage {...handlers} />);
    await userEvent.click(screen.getByRole("button", { name: "Activity" }));
    expect(screen.getByText("Result not recorded")).toBeTruthy();
    expect(screen.getByText("User reported: Worked")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Worked" }));
    expect(handlers.onCompleteSession).toHaveBeenCalledWith("a", "worked");
  });

  it("does not infer that a runtime never ran from a missing log", () => {
    render(<DiagnosticsPage report={null} loading={false} ue4ss={{ installed: true, healthy: true, modCount: 0, logFound: false, logPath: null, extraLoaders: [], vcRuntime: true, protonOverride: null, message: null }} onRun={vi.fn()} onCopy={vi.fn()} onOpenLogs={vi.fn()} onOpenUe4ssLog={vi.fn()} />);
    expect(screen.getByText(/This does not confirm whether the runtime loaded/)).toBeTruthy();
    expect(screen.queryByText(/runtime never ran/)).toBeNull();
  });
});

describe("config preview evidence", () => {
  it("blocks apply when editor content or source file no longer matches the displayed preview", async () => {
    vi.stubGlobal("crypto", { subtle: { digest: vi.fn(async (_algorithm, data: Uint8Array) => new Uint8Array([data.at(-1) ?? 0]).buffer) } });
    const document = { path: "C:/game/settings.ini", format: "ini" as const, content: "value=1", sha256: "before", writable: true };
    const base = props({ configs: [document] });
    const { rerender } = render(<HealthPage {...base} />);
    await userEvent.click(screen.getByRole("button", { name: "Config Workbench" }));
    const editor = screen.getByRole("textbox", { name: "Configuration content" });
    fireEvent.change(editor, { target: { value: "value=2" } });
    const preview = { path: document.path, format: "ini", beforeSha256: "before", afterSha256: "32", diff: ["- 0001 value=1", "+ 0001 value=2"], valid: true, problem: null };
    rerender(<HealthPage {...base} configPreview={preview} />);
    const apply = screen.getByRole("button", { name: "Apply with backup" }) as HTMLButtonElement;
    await waitFor(() => expect(apply.disabled).toBe(false));
    expect(screen.getByText("Preview: 1 added, 1 removed lines")).toBeTruthy();
    fireEvent.change(editor, { target: { value: "value=3" } });
    expect(apply.disabled).toBe(true);
    expect(screen.getByText(/Preview is out of date/)).toBeTruthy();
    expect(base.onApplyConfig).not.toHaveBeenCalled();
    fireEvent.change(editor, { target: { value: "value=2" } });
    await waitFor(() => expect(apply.disabled).toBe(false));
    rerender(<HealthPage {...base} configPreview={{ ...preview, beforeSha256: "stale-source" }} />);
    expect(apply.disabled).toBe(true);
  });
});
