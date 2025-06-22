import { useState, useEffect, useCallback } from "react";
import { ClipboardService } from "../services/clipboardService";
import type { SearchResult } from "../types";

export const useClipboardSearch = () => {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);

  const searchClipboard = useCallback(async (searchQuery: string) => {
    try {
      setLoading(true);
      const searchResults = await ClipboardService.searchClipboard(searchQuery);
      setResults(searchResults);
      setSelectedIndex(0);
    } catch (error) {
      console.error("Search failed:", error);

      if (error && error.toString().includes("not unlocked")) {
        // Handle vault locked case
        throw new Error("VAULT_LOCKED");
      }

      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const copyToClipboard = useCallback(async (content: string, contentType: string) => {
    try {
      await ClipboardService.copyToClipboard(content, contentType);
      return true;
    } catch (error) {
      console.error("Copy failed:", error);
      return false;
    }
  }, []);

  const updateItem = useCallback(async (oldContent: string, newContent: string) => {
    try {
      await ClipboardService.updateItem(oldContent, newContent);
      return true;
    } catch (error) {
      console.error("Update failed:", error);
      return false;
    }
  }, []);

  useEffect(() => {
    searchClipboard(query);
  }, [query, searchClipboard]);

  return {
    query,
    setQuery,
    results,
    selectedIndex,
    setSelectedIndex,
    loading,
    searchClipboard,
    copyToClipboard,
    updateItem,
  };
};