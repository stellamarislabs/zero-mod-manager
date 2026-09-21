// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LegacyImportDialog } from "./LegacyImportDialog";

afterEach(cleanup);

describe("legacy import consent", () => {
  it("imports the library without credentials by default", async () => {
    const onImport = vi.fn();
    render(<LegacyImportDialog status={{ available: true, canImport: true, dataDirectory: "C:/old", libraryDirectory: "C:/old/mods", modCount: 4, fileCount: 19, reason: null }} busy={false} onImport={onImport} onClose={vi.fn()} />);

    expect(screen.getByText(/4 managed mods and 19 library files/)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Import library" }));
    expect(onImport).toHaveBeenCalledWith(false);
  });

  it("copies the Nexus key only after explicit opt-in", async () => {
    const onImport = vi.fn();
    render(<LegacyImportDialog status={{ available: true, canImport: true, dataDirectory: "C:/old", libraryDirectory: "C:/old/mods", modCount: 1, fileCount: 2, reason: null }} busy={false} onImport={onImport} onClose={vi.fn()} />);

    await userEvent.click(screen.getByRole("checkbox", { name: /also import my Nexus API key/i }));
    await userEvent.click(screen.getByRole("button", { name: "Import library" }));
    expect(onImport).toHaveBeenCalledWith(true);
  });
});
