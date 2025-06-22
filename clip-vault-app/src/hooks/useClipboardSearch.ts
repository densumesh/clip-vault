import { useState, useEffect, useCallback, useRef } from "react";
import { ClipboardService } from "../services/clipboardService";
import type { SearchResult } from "../types";

export const useClipboardSearch = () => {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

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

  // Debounced search effect
  useEffect(() => {
    // Cancel previous debounce timer
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    // Cancel previous search request
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    // Set up new debounce timer
    debounceRef.current = setTimeout(() => {
      searchClipboard(query);
    }, query.length === 0 ? 0 : 300); // Immediate for empty query, 300ms for others

    // Cleanup function
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
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