import { describe, expect, it } from "vitest";
import type { ModSummary } from "../types";
import { isUe4ssComponent } from "./runtimeComponents";

const mod = (paths: string[], modType = "ue4ss") => ({ name:"Keybinds", modType, files:paths.map(destination=>({destination})) }) as ModSummary;
describe("bundled component detection", () => {
  it("uses case-insensitive deployed folders, not editable names", () => {
    expect(isUe4ssComponent(mod(["C:\\game\\UE4SS\\Mods\\Keybinds\\Scripts\\main.lua"]))).toBe(true);
    expect(isUe4ssComponent(mod(["/game/ue4ss/Mods/BPML_GenericFunctions/Scripts/main.lua"]))).toBe(true);
    expect(isUe4ssComponent(mod(["/game/ue4ss/Mods/MyCustomMod/Scripts/main.lua"]))).toBe(false);
  });
  it("keeps unknown, mixed and non-UE4SS mods visible", () => {
    expect(isUe4ssComponent(mod([]))).toBe(false);
    expect(isUe4ssComponent(mod(["/game/ue4ss/Mods/Keybinds/main.lua", "/game/custom.lua"]))).toBe(false);
    expect(isUe4ssComponent(mod(["/game/ue4ss/Mods/Keybinds/main.lua"], "pak"))).toBe(false);
  });
});
