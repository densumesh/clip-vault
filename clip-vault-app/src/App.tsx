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
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [password, setPassword] = useState("");
  const [vaultUnlocked, setVaultUnlocked] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);

  const checkVaultStatus = async () => {
    try {
      const isUnlocked = await invoke<boolean>("check_vault_status");
      setVaultUnlocked(isUnlocked);

      if (!isUnlocked) {
        setShowPasswordPrompt(true);
        setResults([]);
      }

      return isUnlocked;
    } catch (error) {
      console.error("Failed to check vault status:", error);
      setVaultUnlocked(false);
      setShowPasswordPrompt(true);
      return false;
    }
  };

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

      // If search fails due to vault being locked, trigger unlock prompt
      if (error && error.toString().includes("not unlocked")) {
        setVaultUnlocked(false);
        setShowPasswordPrompt(true);
      }

      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  const copyToClipboard = async (content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      const window = getCurrentWebviewWindow();
      await window.hide();
    } catch (error) {
      console.error("Copy failed:", error);
    }
  };


  const handleUnlock = async () => {
    try {
      const success = await invoke<boolean>("unlock_vault", { password });
      if (success) {
        setVaultUnlocked(true);
        setShowPasswordPrompt(false);
        setPassword("");
        // Refresh search results
        searchClipboard(query);
        searchInputRef.current?.focus();
      } else {
        alert("Invalid password. Please try again.");
      }
    } catch (error) {
      console.error("Unlock failed:", error);
      alert("Failed to unlock vault.");
    }
  };

  const formatTimestamp = (timestamp: number) => {
    const date = new Date(timestamp / 1_000_000);
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

  const getWindowedContent = (content: string, query: string, maxLength: number = 100) => {
    if (!query.trim()) {
      // No search query, just truncate normally
      if (content.length <= maxLength) return content;
      return content.substring(0, maxLength) + "...";
    }

    // Find the first match
    const regex = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi');
    const match = content.match(regex);

    if (!match) {
      // No match found, truncate normally
      if (content.length <= maxLength) return content;
      return content.substring(0, maxLength) + "...";
    }

    const matchIndex = content.toLowerCase().indexOf(match[0].toLowerCase());

    if (content.length <= maxLength) {
      // Content is short enough, return as is
      return content;
    }

    // Calculate window around the match
    const contextLength = Math.floor((maxLength - match[0].length) / 2);
    let start = Math.max(0, matchIndex - contextLength);
    let end = Math.min(content.length, matchIndex + match[0].length + contextLength);

    // Adjust if we hit boundaries
    if (end - start < maxLength) {
      if (start === 0) {
        end = Math.min(content.length, maxLength);
      } else if (end === content.length) {
        start = Math.max(0, content.length - maxLength);
      }
    }

    // Try to break at word boundaries
    if (start > 0) {
      const spaceIndex = content.lastIndexOf(' ', start + 20);
      if (spaceIndex > start && spaceIndex < start + 20) {
        start = spaceIndex + 1;
      }
    }

    if (end < content.length) {
      const spaceIndex = content.indexOf(' ', end - 20);
      if (spaceIndex !== -1 && spaceIndex < end + 20) {
        end = spaceIndex;
      }
    }

    const windowed = content.substring(start, end);
    const prefix = start > 0 ? "..." : "";
    const suffix = end < content.length ? "..." : "";

    return prefix + windowed + suffix;
  };

  const highlightText = (text: string, query: string) => {
    if (!query.trim()) return text;

    const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    const parts = text.split(regex);

    return parts.map((part, index) =>
      regex.test(part) ? (
        <mark key={index} className="highlight">{part}</mark>
      ) : (
        part
      )
    );
  };

  useEffect(() => {
    const initializeApp = async () => {
      const isUnlocked = await checkVaultStatus();
      if (isUnlocked) {
        searchClipboard("");
        searchInputRef.current?.focus();
      }
    };

    initializeApp();
  }, []);

  useEffect(() => {
    searchClipboard(query);
  }, [query]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't handle shortcuts if modals are open
      if (showPasswordPrompt) return;

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
  }, [results, selectedIndex, showPasswordPrompt]);

  useEffect(() => {
    const activeEl = resultRefs.current[selectedIndex];
    if (activeEl) {
      activeEl.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  }, [selectedIndex]);

  useEffect(() => {
    const handleWindowBlur = () => {
      const window = getCurrentWebviewWindow();
      window.hide();
    };

    const handleWindowFocus = async () => {
      // Check vault status when window gains focus
      await checkVaultStatus();
    };

    window.addEventListener('blur', handleWindowBlur);
    window.addEventListener('focus', handleWindowFocus);

    return () => {
      window.removeEventListener('blur', handleWindowBlur);
      window.removeEventListener('focus', handleWindowFocus);
    };
  }, []);


  return (
    <div className="app">
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
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search your clipboard history..."
          className="search-input"
        />
        {!loading && query !== "" && (
          <div className="results-count">
            {results.length === 0
              ? (query ? "No matches" : "No items")
              : `${results.length} result${results.length === 1 ? '' : 's'}`
            }
          </div>
        )}
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
                  {highlightText(getWindowedContent(result.content, query), query)}
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

      <div className="help-text">
        Use ↑↓ to navigate • Enter to copy • Esc to close
      </div>

      {/* Password Prompt Modal */}
      {showPasswordPrompt && (
        <div className="modal-overlay">
          <div className="modal">
            <div className="modal-header">
              <h2>Unlock Vault</h2>
            </div>
            <div className="modal-content">
              <p>Enter your vault password to unlock the database:</p>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleUnlock();
                  } else if (e.key === "Escape") {
                    setShowPasswordPrompt(false);
                    setPassword("");
                  }
                }}
                placeholder="Vault password"
                autoFocus
              />
            </div>
            <div className="modal-footer">
              <button
                className="modal-button secondary"
                onClick={() => {
                  setShowPasswordPrompt(false);
                  setPassword("");
                }}
              >
                Cancel
              </button>
              <button
                className="modal-button primary"
                onClick={handleUnlock}
              >
                Unlock
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
