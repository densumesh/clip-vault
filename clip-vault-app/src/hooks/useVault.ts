import { useState, useEffect, useCallback } from "react";
import { VaultService } from "../services/vaultService";
import { ClipboardService } from "../services/clipboardService";
import type { OnboardingSettings } from "../components";

export const useVault = () => {
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [password, setPassword] = useState("");

  const checkVaultStatus = useCallback(async () => {
    try {
      // First check if vault exists
      const vaultExists = await ClipboardService.vaultExists();
      
      if (!vaultExists) {
        setShowOnboarding(true);
        setShowPasswordPrompt(false);
        setIsUnlocked(false);
        return false;
      }

      const unlocked = await VaultService.checkVaultStatus();

      if (!unlocked) {
        setShowPasswordPrompt(true);
        setShowOnboarding(false);
        setIsUnlocked(false);
      } else {
        setIsUnlocked(true);
        setShowPasswordPrompt(false);
        setShowOnboarding(false);
      }

      return unlocked;
    } catch (error) {
      console.error("Failed to check vault status:", error);
      setShowPasswordPrompt(true);
      setShowOnboarding(false);
      setIsUnlocked(false);
      return false;
    }
  }, []);

  const unlockVault = useCallback(async (vaultPassword: string) => {
    try {
      const success = await VaultService.unlockVault(vaultPassword);
      if (success) {
        setShowPasswordPrompt(false);
        setPassword("");
        setIsUnlocked(true);
        return true;
      } else {
        return false;
      }
    } catch (error) {
      console.error("Unlock failed:", error);
      return false;
    }
  }, []);

  const handleUnlock = useCallback(async () => {
    const success = await unlockVault(password);
    if (!success) {
      alert("Invalid password. Please try again.");
    }
    return success;
  }, [password, unlockVault]);

  const handleCancel = useCallback(() => {
    setShowPasswordPrompt(false);
    setPassword("");
  }, []);

  const handleOnboardingComplete = useCallback(async (settings: OnboardingSettings) => {
    try {
      // Get current settings from backend (which includes proper vault path)
      const currentSettings = await VaultService.getSettings();
      
      // Update only the settings we care about from onboarding
      const updatedSettings = {
        ...currentSettings,
        auto_lock_minutes: settings.sessionTimeMinutes,
        global_shortcut: settings.keyCombo,
      };

      const success = await ClipboardService.createVault(settings.password, updatedSettings);
      
      if (success) {
        setShowOnboarding(false);
        setIsUnlocked(true);
        return true;
      } else {
        alert("Failed to create vault. Please try again.");
        return false;
      }
    } catch (error) {
      console.error("Failed to complete onboarding:", error);
      alert("Failed to create vault. Please try again.");
      return false;
    }
  }, []);

  useEffect(() => {
    checkVaultStatus();
  }, [checkVaultStatus]);

  return {
    isUnlocked,
    showPasswordPrompt,
    showOnboarding,
    password,
    setPassword,
    checkVaultStatus,
    unlockVault,
    handleUnlock,
    handleCancel,
    handleOnboardingComplete,
  };
};