import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./App.css";

interface SearchResult {
  id: string;
  content: string;
  timestamp: number;
  content_type: string;
}

function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);

  const searchClipboard = async (searchQuery: string) => {
    try {
      setLoading(true);
      const searchResults = await invoke<SearchResult[]>("search_clipboard", {
        query: searchQuery,
      });
      setResults(searchResults);
      setSelectedIndex(0);
    } catch (error) {
      console.error("Search failed:", error);
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  const copyToClipboard = async (content: string) => {
    try {
      await invoke("copy_to_clipboard", { content });
      setCopied(true);
      setTimeout(() => setCopied(false), 1000);

      const window = getCurrentWebviewWindow();
      await window.hide();
    } catch (error) {
      console.error("Copy failed:", error);
    }
  };

  const formatTimestamp = (timestamp: number) => {

    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 1) return "just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const truncateContent = (content: string, maxLength: number = 100) => {
    if (content.length <= maxLength) return content;
    return content.substring(0, maxLength) + "...";
  };

  useEffect(() => {
    searchClipboard("");
    if (searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, []);

  useEffect(() => {
    searchClipboard(query);
  }, [query]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        const window = getCurrentWebviewWindow();
        window.hide();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (results[selectedIndex]) {
          copyToClipboard(results[selectedIndex].content);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [results, selectedIndex]);

  useEffect(() => {
    const activeEl = resultRefs.current[selectedIndex];
    if (activeEl) {
      activeEl.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  }, [selectedIndex]);

  return (
    <div className="app">
      <div className="search-container">
        <input
          ref={searchInputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search your clipboard history..."
          className="search-input"
        />
      </div>

      <div className="results-container">
        {loading && <div className="loading">Searching...</div>}

        {!loading && results.length === 0 && (
          <div className="empty-state">
            {query ? "No matches found" : "No clipboard history yet"}
          </div>
        )}

        {!loading && results.length > 0 && (
          <div className="results-list">
            {results.map((result, index) => (
              <div
                key={result.id}
                ref={el => (resultRefs.current[index] = el)}
                className={`result-item ${index === selectedIndex ? "selected" : ""}`}
                onClick={() => copyToClipboard(result.content)}
              >
                <div className="result-content">
                  {truncateContent(result.content)}
                </div>
                <div className="result-meta">
                  <span className="result-time">
                    {formatTimestamp(result.timestamp)}
                  </span>
                  <span className="result-type">{result.content_type}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {copied && (
        <div className="copy-notification">
          Copied to clipboard!
        </div>
      )}

      <div className="help-text">
        Use ↑↓ to navigate • Enter to copy • Esc to close
      </div>
    </div>
  );
}

export default App;
