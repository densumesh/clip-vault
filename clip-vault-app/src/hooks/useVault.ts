import { useState, useEffect, useCallback } from "react";
import { VaultService } from "../services/vaultService";

export const useVault = () => {
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [password, setPassword] = useState("");

  const checkVaultStatus = useCallback(async () => {
    try {
      const unlocked = await VaultService.checkVaultStatus();

      if (!unlocked) {
        setShowPasswordPrompt(true);
        setIsUnlocked(false);
      } else {
        setIsUnlocked(true);
        setShowPasswordPrompt(false);
      }

      return unlocked;
    } catch (error) {
      console.error("Failed to check vault status:", error);
      setShowPasswordPrompt(true);
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

  useEffect(() => {
    checkVaultStatus();
  }, [checkVaultStatus]);

  return {
    isUnlocked,
    showPasswordPrompt,
    password,
    setPassword,
    checkVaultStatus,
    unlockVault,
    handleUnlock,
    handleCancel,
  };
};