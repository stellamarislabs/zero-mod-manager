import { describe, expect, it } from "vitest";
import { changedManagedFile, friendlyError, isChangedFileError } from "./backend";

describe("friendlyError", () => {
  it("passes a backend message through unchanged", () => {
    expect(friendlyError("Nexus Mods rejected the API key. Check it in Settings.")).toBe(
      "Nexus Mods rejected the API key. Check it in Settings."
    );
  });

  it("falls back to a readable sentence for a value that carries no message", () => {
    expect(friendlyError({ unexpected: true })).toBe("The operation could not be completed.");
  });
});

describe("isChangedFileError", () => {
  /**
   * The prefix is the contract with `AppError::ChecksumMismatch`. If the Rust
   * message is reworded without updating this, the interface silently loses
   * the override and the mod becomes impossible to update or remove again.
   */
  it("recognises the guard on a managed file that changed on disk", () => {
    const message =
      "A managed file changed outside Zero Mod Manager: " +
      "D:\\Games\\SWZeroCompany\\Binaries\\Win64\\ue4ss\\Mods\\ConfigManager\\registry.txt. " +
      "Some mods write their own settings or data files there while the game runs.";
    expect(isChangedFileError(message)).toBe(true);
    expect(isChangedFileError(new Error(message))).toBe(true);
    expect(changedManagedFile(message)).toBe("D:\\Games\\SWZeroCompany\\Binaries\\Win64\\ue4ss\\Mods\\ConfigManager\\registry.txt");
  });

  it("recognises a real transaction rollback before offering an explicit overwrite", () => {
    const path = "C:\\Games\\SWZeroCompany\\Binaries\\Win64\\ue4ss\\Mods\\HelmetHide\\dlls\\main.dll";
    const message = `No package changes were kept. Previous files and library were restored. A managed file changed outside Zero Mod Manager: ${path}`;
    expect(changedManagedFile(message)).toBe(path);
    expect(isChangedFileError(message)).toBe(true);
    expect(changedManagedFile(`Operation failed: A managed file changed outside Zero Mod Manager: ${path}. Recovery pending: disk error.`)).toBeNull();
  });

  it("leaves every other failure to the ordinary error path", () => {
    expect(isChangedFileError("A different file already exists at C:\\x.pak. It was not overwritten.")).toBe(false);
    expect(isChangedFileError("The installation preview expired. Inspect the mod again.")).toBe(false);
    expect(isChangedFileError(null)).toBe(false);
  });
});
