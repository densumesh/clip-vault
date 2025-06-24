import { invoke } from "@tauri-apps/api/core";
import type { SearchResult } from "../types";
import { cacheService } from "./cacheService";

export class ClipboardService {
  static async listClipboard(
    limit?: number,
    afterTimestamp?: number
  ): Promise<{ results: SearchResult[]; hasMore: boolean }> {
    try {
      // Check cache first
      const cached = cacheService.getList(limit, afterTimestamp);
      if (cached) {
        return { results: cached.data, hasMore: cached.hasMore };
      }

      // Fetch from backend
      const results = await invoke<SearchResult[]>("list_clipboard", {
        limit,
        afterTimestamp,
      });

      // Determine if there are more results
      const hasMore = results.length === (limit || 20);

      // Cache the results
      cacheService.setList(results, hasMore, limit, afterTimestamp);

      return { results, hasMore };
    } catch (error) {
      console.error("List failed:", error);
      throw error;
    }
  }

  static async searchClipboard(
    query: string,
    limit?: number,
    afterTimestamp?: number
  ): Promise<{ results: SearchResult[]; hasMore: boolean }> {
    try {
      // Check cache first
      const cached = cacheService.getSearch(query, limit, afterTimestamp);
      if (cached) {
        return { results: cached.data, hasMore: cached.hasMore };
      }

      // Fetch from backend
      const searchResults = await invoke<SearchResult[]>("search_clipboard", {
        query,
        limit,
        afterTimestamp,
      });

      // Determine if there are more results
      const hasMore = searchResults.length === (limit || 20);

      // Cache the results
      cacheService.setSearch(query, searchResults, hasMore, limit, afterTimestamp);

      return { results: searchResults, hasMore };
    } catch (error) {
      console.error("Search failed:", error);
      throw error;
    }
  }

  /**
   * Invalidate cache when new clipboard item is added
   */
  static invalidateCache(): void {
    cacheService.invalidateAll();
  }

  static async copyToClipboard(content: string, contentType: string): Promise<void> {
    try {
      await invoke("copy_to_clipboard", {
        content,
        contentType,
      });
    } catch (error) {
      console.error("Copy failed:", error);
      throw error;
    }
  }

  static async updateItem(oldContent: string, newContent: string): Promise<void> {
    try {
      await invoke("update_item", {
        oldContent,
        newContent,
      });
    } catch (error) {
      console.error("Failed to update item:", error);
      throw error;
    }
  }

  static async deleteItem(content: string): Promise<void> {
    try {
      await invoke("delete_item", { content });
    } catch (error) {
      console.error("Failed to delete item:", error);
      throw error;
    }
  }
}