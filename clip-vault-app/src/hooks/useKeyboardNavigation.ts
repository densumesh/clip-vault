import { useEffect, useCallback } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { SearchResult } from "../types";

interface UseKeyboardNavigationProps {
  results: SearchResult[];
  selectedIndex: number;
  setSelectedIndex: (index: number) => void;
  onCopy: (content: string, contentType: string) => void;
  showPasswordPrompt: boolean;
  setSearching: (searching: boolean) => void;
  setQuery: (query: string) => void;
}

export const useKeyboardNavigation = ({
  results,
  selectedIndex,
  setSelectedIndex,
  onCopy,
  showPasswordPrompt,
  setSearching,
  setQuery,
}: UseKeyboardNavigationProps) => {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    // Don't handle shortcuts if modals are open
    if (showPasswordPrompt) return;

    if (e.key === "Escape") {
      // Clear search query when closing
      setQuery("");
      const window = getCurrentWebviewWindow();
      window.hide();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex(Math.min(selectedIndex + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex(Math.max(selectedIndex - 1, 0));
    } else if (e.key === "Enter" || (e.metaKey && e.key === "c")) {
      e.preventDefault();
      if (results[selectedIndex]) {
        onCopy(results[selectedIndex].content, results[selectedIndex].content_type);
      }
    } else if (e.key === "/") {
      e.preventDefault();
      setSearching(true);
    }
  }, [results, selectedIndex, setSelectedIndex, onCopy, showPasswordPrompt, setQuery]);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
};