import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

export class VaultService {
  static async checkVaultStatus(): Promise<boolean> {
    try {
      const isUnlocked = await invoke<boolean>("check_vault_status");
      return isUnlocked;
    } catch (error) {
      console.error("Failed to check vault status:", error);
      return false;
    }
  }

  static async unlockVault(password: string): Promise<boolean> {
    try {
      const success = await invoke<boolean>("unlock_vault", { password });
      return success;
    } catch (error) {
      console.error("Unlock failed:", error);
      return false;
    }
  }

  static async getSettings(): Promise<AppSettings> {
    try {
      const settings = await invoke<AppSettings>("get_settings");
      return settings;
    } catch (error) {
      console.error("Failed to get settings:", error);
      throw error;
    }
  }

  static async saveSettings(settings: AppSettings): Promise<void> {
    try {
      await invoke("save_settings", { newSettings: settings });
    } catch (error) {
      console.error("Failed to save settings:", error);
      throw error;
    }
  }

  static async startDaemon(): Promise<void> {
    try {
      await invoke("start_daemon");
    } catch (error) {
      console.error("Failed to start daemon:", error);
      throw error;
    }
  }

  static async stopDaemon(): Promise<void> {
    try {
      await invoke("stop_daemon");
    } catch (error) {
      console.error("Failed to stop daemon:", error);
      throw error;
    }
  }

  static async getDaemonStatus(): Promise<boolean> {
    try {
      const isRunning = await invoke<boolean>("daemon_status");
      return isRunning;
    } catch (error) {
      console.error("Failed to get daemon status:", error);
      return false;
    }
  }
}