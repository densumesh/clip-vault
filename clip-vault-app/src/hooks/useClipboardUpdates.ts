import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { ClipboardService } from "../services/clipboardService";

interface UseClipboardUpdatesProps {
  onClipboardUpdate: () => void;
  showPasswordPrompt: boolean;
}

export const useClipboardUpdates = ({ onClipboardUpdate, showPasswordPrompt }: UseClipboardUpdatesProps) => {
  useEffect(() => {
    if (showPasswordPrompt) return;

    let unlisten: (() => void) | undefined;

    const setupEventListener = async () => {
      try {
        unlisten = await listen("clipboard-updated", () => {
          console.log("Clipboard updated, invalidating cache and refreshing results...");
          // Invalidate all cached data since new content was added
          ClipboardService.invalidateCache();
          onClipboardUpdate();
        });
      } catch (error) {
        console.error("Failed to setup clipboard event listener:", error);
      }
    };

    setupEventListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [onClipboardUpdate, showPasswordPrompt]);
};