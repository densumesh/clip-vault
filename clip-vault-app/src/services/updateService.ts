import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export class UpdateService {
  static async checkForUpdates(): Promise<string | null> {
    try {
      return await invoke<string | null>("check_for_updates");
    } catch (error) {
      console.error("Failed to check for updates:", error);
      throw error;
    }
  }

  static async installUpdate(): Promise<void> {
    try {
      await invoke("install_update");
    } catch (error) {
      console.error("Failed to install update:", error);
      throw error;
    }
  }

  static async listenToUpdateProgress(callback: (progress: number) => void) {
    return await listen<number>("update-progress", (event) => {
      callback(event.payload);
    });
  }

  static async listenToUpdateInstalled(callback: () => void) {
    return await listen("update-installed", () => {
      callback();
    });
  }
}
