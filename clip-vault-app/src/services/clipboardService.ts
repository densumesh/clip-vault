import { invoke } from "@tauri-apps/api/core";
import type { SearchResult } from "../types";

export class ClipboardService {
  static async searchClipboard(query: string): Promise<SearchResult[]> {
    try {
      const searchResults = await invoke<SearchResult[]>("search_clipboard", {
        query,
      });
      return searchResults;
    } catch (error) {
      console.error("Search failed:", error);
      throw error;
    }
  }

  static async copyToClipboard(content: string, contentType?: string): Promise<void> {
    try {
      if (contentType && contentType.startsWith('image/')) {
        await navigator.clipboard.write([
          new ClipboardItem({
            [contentType]: new Blob([atob(content)], { type: contentType })
          })
        ]);
      } else {
        await navigator.clipboard.writeText(content);
      }
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

  static async deleteItem(timestamp: number): Promise<void> {
    try {
      await invoke("delete_item", { timestamp });
    } catch (error) {
      console.error("Failed to delete item:", error);
      throw error;
    }
  }
}