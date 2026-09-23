// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { DiagnosticsPage } from "./DiagnosticsPage";

afterEach(cleanup);
it("uses actual findings rather than an old summary string to color diagnostics", () => {
  const { container } = render(<DiagnosticsPage report={{ overall: "GOOD", text: "", items: [{ label: "Game build", status: "unknown", value: "Unavailable", action: null }] }} loading={false} ue4ss={null} onRun={vi.fn()} onCopy={vi.fn()} onOpenUe4ssLog={vi.fn()} onOpenLogs={vi.fn()} />);
  expect(screen.getByText("No issues found by these checks")).toBeTruthy();
  expect(container.querySelector(".doctor-summary.warning")).toBeNull();
  expect(screen.getByText("Unavailable")).toBeTruthy();
});
