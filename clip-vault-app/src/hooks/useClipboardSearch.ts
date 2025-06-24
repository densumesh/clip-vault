import { useState, useEffect, useCallback, useRef } from "react";
import { ClipboardService } from "../services/clipboardService";
import { cacheService } from "../services/cacheService";
import type { SearchResult } from "../types";

export const useClipboardSearch = () => {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

  const searchClipboard = useCallback(async (searchQuery: string) => {
    try {
      setLoading(true);
      const response = searchQuery.trim() === ""
        ? await ClipboardService.listClipboard(20)
        : await ClipboardService.searchClipboard(searchQuery, 20);
      setResults(response.results);
      setSelectedIndex(0);
      setHasMore(response.hasMore);
    } catch (error) {
      console.error("Search failed:", error);

      if (error && error.toString().includes("not unlocked")) {
        // Handle vault locked case
        throw new Error("VAULT_LOCKED");
      }

      setResults([]);
      setHasMore(false);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadMore = useCallback(async () => {
    if (!hasMore || loadingMore || results.length === 0) return;

    try {
      setLoadingMore(true);
      const lastTimestamp = results[results.length - 1].timestamp;
      const response = query.trim() === ""
        ? await ClipboardService.listClipboard(20, lastTimestamp)
        : await ClipboardService.searchClipboard(query, 20, lastTimestamp);

      if (response.results.length > 0) {
        const existingTimestamps = new Set(results.map(r => r.timestamp));
        const newResults = response.results.filter(result => !existingTimestamps.has(result.timestamp));

        if (newResults.length > 0) {
          setResults(prev => [...prev, ...newResults]);
        }
        setHasMore(response.hasMore);
      } else {
        setHasMore(false);
      }
    } catch (error) {
      console.error("Load more failed:", error);
      setHasMore(false);
    } finally {
      setLoadingMore(false);
    }
  }, [query, results, hasMore, loadingMore]);

  const copyToClipboard = useCallback(async (content: string, contentType: string) => {
    try {
      await ClipboardService.copyToClipboard(content, contentType);
      
      // Only do optimistic update if we're viewing the main list (not searching)
      if (query.trim() === "") {
        // Optimistically update the list - find the copied item and move it to the top
        setResults(prevResults => {
          const copiedItemIndex = prevResults.findIndex(item => 
            item.content === content && item.content_type === contentType
          );
          
          if (copiedItemIndex > 0) {
            // Create new array with the copied item moved to the front
            const newResults = [...prevResults];
            const [copiedItem] = newResults.splice(copiedItemIndex, 1);
            
            // Update timestamp to current time for realistic ordering
            const updatedItem = {
              ...copiedItem,
              timestamp: Date.now() * 1000000 // Convert to nanoseconds like backend
            };
            
            return [updatedItem, ...newResults];
          }
          
          return prevResults;
        });
        
        // Reset selected index to 0 since the copied item is now at the top
        setSelectedIndex(0);
      }
      
      // Invalidate cache since we've made an optimistic update
      // The real clipboard-updated event will refresh with accurate data
      cacheService.invalidateAll();
      
      return true;
    } catch (error) {
      console.error("Copy failed:", error);
      return false;
    }
  }, [query]);

  const updateItem = useCallback(async (oldContent: string, newContent: string) => {
    try {
      await ClipboardService.updateItem(oldContent, newContent);
      return true;
    } catch (error) {
      console.error("Update failed:", error);
      return false;
    }
  }, []);

  const deleteItem = useCallback(async (content: string) => {
    try {
      await ClipboardService.deleteItem(content);
      
      // Optimistically update the list - remove the deleted item
      setResults(prevResults => 
        prevResults.filter(item => item.content !== content)
      );
      
      // Reset selected index if we deleted the selected item
      setSelectedIndex(prev => {
        const deletedIndex = results.findIndex(item => item.content === content);
        if (deletedIndex === prev) {
          return Math.max(0, prev - 1);
        } else if (deletedIndex < prev) {
          return prev - 1;
        }
        return prev;
      });
      
      // Invalidate cache since we've made changes
      cacheService.invalidateAll();
      
      return true;
    } catch (error) {
      console.error("Delete failed:", error);
      return false;
    }
  }, [results]);

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

    // Reset pagination state when query changes
    setHasMore(true);
    setLoadingMore(false);

    // Set up new debounce timer
    debounceRef.current = setTimeout(() => {
      searchClipboard(query);
    }, query.length === 0 ? 0 : 100); // Immediate for empty query, 300ms for others

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
    loadingMore,
    hasMore,
    searchClipboard,
    loadMore,
    copyToClipboard,
    updateItem,
    deleteItem,
  };
};