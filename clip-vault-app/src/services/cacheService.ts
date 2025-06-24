import type { SearchResult } from "../types";

interface CacheEntry {
  data: SearchResult[];
  timestamp: number;
  hasMore: boolean;
}

interface CacheKey {
  type: 'list' | 'search';
  query?: string;
  limit?: number;
  afterTimestamp?: number;
}

class ClipboardCacheService {
  private cache = new Map<string, CacheEntry>();
  private readonly CACHE_TTL = 5 * 60 * 1000; // 5 minutes
  private readonly MAX_CACHE_SIZE = 100; // Maximum number of cached entries

  /**
   * Generate a unique cache key based on the query parameters
   */
  private generateKey(key: CacheKey): string {
    const parts: string[] = [key.type];
    if (key.query !== undefined) parts.push(`q:${key.query}`);
    if (key.limit !== undefined) parts.push(`l:${key.limit}`);
    if (key.afterTimestamp !== undefined) parts.push(`t:${key.afterTimestamp}`);
    return parts.join('|');
  }

  /**
   * Check if a cache entry is still valid
   */
  private isValid(entry: CacheEntry): boolean {
    const now = Date.now();
    return (now - entry.timestamp) < this.CACHE_TTL;
  }

  /**
   * Clean up expired entries and enforce size limit
   */
  private cleanup(): void {
    const now = Date.now();
    
    // Remove expired entries
    for (const [key, entry] of this.cache.entries()) {
      if ((now - entry.timestamp) >= this.CACHE_TTL) {
        this.cache.delete(key);
      }
    }

    // Enforce size limit by removing oldest entries
    if (this.cache.size > this.MAX_CACHE_SIZE) {
      const entries = Array.from(this.cache.entries())
        .sort(([, a], [, b]) => a.timestamp - b.timestamp);
      
      const toRemove = entries.slice(0, this.cache.size - this.MAX_CACHE_SIZE);
      toRemove.forEach(([key]) => this.cache.delete(key));
    }
  }

  /**
   * Get cached results for a list request
   */
  getList(limit?: number, afterTimestamp?: number): CacheEntry | null {
    const key = this.generateKey({ type: 'list', limit, afterTimestamp });
    const entry = this.cache.get(key);
    
    if (entry && this.isValid(entry)) {
      return entry;
    }
    
    return null;
  }

  /**
   * Get cached results for a search request
   */
  getSearch(query: string, limit?: number, afterTimestamp?: number): CacheEntry | null {
    const key = this.generateKey({ type: 'search', query, limit, afterTimestamp });
    const entry = this.cache.get(key);
    
    if (entry && this.isValid(entry)) {
      return entry;
    }
    
    return null;
  }

  /**
   * Cache list results
   */
  setList(data: SearchResult[], hasMore: boolean, limit?: number, afterTimestamp?: number): void {
    const key = this.generateKey({ type: 'list', limit, afterTimestamp });
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      hasMore
    });
    this.cleanup();
  }

  /**
   * Cache search results
   */
  setSearch(query: string, data: SearchResult[], hasMore: boolean, limit?: number, afterTimestamp?: number): void {
    const key = this.generateKey({ type: 'search', query, limit, afterTimestamp });
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      hasMore
    });
    this.cleanup();
  }

  /**
   * Invalidate all cache entries (called when new clipboard item is added)
   */
  invalidateAll(): void {
    this.cache.clear();
  }

  /**
   * Invalidate only list caches (keep search caches)
   */
  invalidateList(): void {
    for (const [key] of this.cache.entries()) {
      if (key.startsWith('list|')) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Invalidate specific search caches
   */
  invalidateSearch(query?: string): void {
    if (query) {
      for (const [key] of this.cache.entries()) {
        if (key.startsWith('search|') && key.includes(`q:${query}`)) {
          this.cache.delete(key);
        }
      }
    } else {
      // Invalidate all search caches
      for (const [key] of this.cache.entries()) {
        if (key.startsWith('search|')) {
          this.cache.delete(key);
        }
      }
    }
  }

  /**
   * Get cache statistics for debugging
   */
  getStats(): { size: number; keys: string[] } {
    return {
      size: this.cache.size,
      keys: Array.from(this.cache.keys())
    };
  }

  /**
   * Manually clear all cache
   */
  clear(): void {
    this.cache.clear();
  }
}

// Export a singleton instance
export const cacheService = new ClipboardCacheService();