import React, { useRef, useEffect } from "react";
import type { SearchInputProps } from "../types";

export const SearchInput: React.FC<SearchInputProps> = ({
  query,
  onQueryChange,
  loading,
  resultsCount,
}) => {
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Focus input when component mounts
    searchInputRef.current?.focus();
  }, []);

  return (
    <div className="search-container" data-tauri-drag-region>
      <div className="drag-handle">
        <div className="drag-dots">
          <span></span>
          <span></span>
          <span></span>
        </div>
      </div>
      <input
        ref={searchInputRef}
        type="text"
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        placeholder="Search your clipboard history..."
        className="search-input"
      />
      {!loading && query !== "" && (
        <div className="results-count">
          {resultsCount === 0
            ? (query ? "No matches" : "No items")
            : `${resultsCount} result${resultsCount === 1 ? '' : 's'}`
          }
        </div>
      )}
    </div>
  );
};