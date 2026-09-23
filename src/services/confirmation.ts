import { confirm } from "@tauri-apps/plugin-dialog";

/** Never treat an asynchronous native dialog Promise as user consent. */
export async function requestConfirmation(message: string, onFailure: (error: unknown) => void, okLabel = "Continue"): Promise<boolean> {
  try {
    return (await confirm(message, { title: "Zero Mod Manager", kind: "warning", okLabel, cancelLabel: "Cancel" })) === true;
  } catch (error) {
    onFailure(error);
    return false;
  }
}
