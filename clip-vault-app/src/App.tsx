import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';
import "./App.css";

interface SearchResult {
  id: string;
  content: string;
  timestamp: number;
  content_type: string;
}

interface PreviewPaneProps {
  selectedItem: SearchResult | null;
  onCopy: (content: string, contentType?: string) => void;
  onEdit?: () => void;
}

const PreviewPane: React.FC<PreviewPaneProps> = ({ selectedItem, onCopy }) => {
  const previewRef = useRef<HTMLElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [language, setLanguage] = useState<string>("");
  const [isEditing, setIsEditing] = useState(false);
  const [editedContent, setEditedContent] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp / 1_000_000);
    return date.toLocaleString();
  };

  const getContentStats = (content: string) => {
    const lines = content.split('\n').length;
    const chars = content.length;
    const words = content.trim().split(/\s+/).length;
    return { lines, chars, words };
  };

  const handleSave = async () => {
    if (!selectedItem || isSaving) return;

    try {
      setIsSaving(true);
      await invoke("update_item", {
        oldContent: selectedItem.content,
        newContent: editedContent,
      });
      setIsEditing(false);
      // Note: The backend should emit a clipboard-updated event to refresh results
    } catch (error) {
      console.error("Failed to save item:", error);
      alert("Failed to save changes. Please try again.");
    } finally {
      setIsSaving(false);
    }
  };

  const handleCancel = () => {
    setIsEditing(false);
    setEditedContent(selectedItem?.content || "");
  };

  const handleEdit = () => {
    setIsEditing(true);
    setTimeout(() => {
      if (textareaRef.current) {
        textareaRef.current.focus();
      }
    }, 0);
  };

  useEffect(() => {
    const highlightContent = async () => {
      if (!selectedItem || !previewRef.current) return;
      if (selectedItem.content_type.startsWith("image/")) {
        setLanguage("image");
        previewRef.current.innerHTML = "";
        return;
      }

      const highlighted = hljs.highlightAuto(selectedItem.content);
      if (highlighted.relevance > 10) {
        previewRef.current.innerHTML = highlighted.value;
        setLanguage(highlighted.language || "plaintext");
      } else {
        previewRef.current.innerText = selectedItem.content;
        setLanguage("plaintext");
      }
      setIsEditing(false);
      setEditedContent(selectedItem?.content || "");
    }
    highlightContent();
  }, [selectedItem]);

  // Handle keyboard shortcuts in edit mode
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if (e.key === 's' && isEditing) {
          e.preventDefault();
          handleSave();
        } else if (e.key === 'e' && !isEditing && selectedItem?.content_type.startsWith('text')) {
          e.preventDefault();
          handleEdit();
        }
      } else if (e.key === 'Escape' && isEditing) {
        e.preventDefault();
        handleCancel();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isEditing, editedContent, selectedItem]);

  if (!selectedItem) {
    return (
      <div className="preview-pane">
        <div className="preview-empty">
          <div className="preview-empty-icon">📋</div>
          <div className="preview-empty-text">Select an item to preview</div>
        </div>
      </div>
    );
  }

  const stats = getContentStats(selectedItem.content);

  return (
    <div className="preview-pane">
      <div className="preview-header">
        <div className="preview-metadata">
          <div className="preview-timestamp">
            {formatTimestamp(selectedItem.timestamp)}
          </div>
          {language != "image" ? (
            <div className="preview-stats">
              <span className="stat-item">{stats.lines} lines</span>
              <span className="stat-item">{stats.chars} chars</span>
              <span className="stat-item">{stats.words} words</span>
              {language && <span className="stat-item language-tag">{language}</span>}
            </div>
          ) : (
            <div className="preview-stats">
              <span className="stat-item">{(() => {
                const img = new Image();
                img.src = `data:${selectedItem.content_type};base64,${selectedItem.content}`;
                return `${img.width} × ${img.height}px`;
              })()}</span>
              <span className="stat-item language-tag">Image</span>
            </div>
          )}
        </div>
        <div className="preview-actions">
          {isEditing ? (
            <>
              <button
                className="preview-button save"
                onClick={handleSave}
                disabled={isSaving || editedContent === selectedItem.content}
                title="Save changes (Ctrl+S)"
              >
                {isSaving ? "⏳ Saving..." : "💾 Save"}
              </button>
              <button
                className="preview-button cancel"
                onClick={handleCancel}
                disabled={isSaving}
                title="Cancel editing (Esc)"
              >
                ✖ Cancel
              </button>
            </>
          ) : (
            <>
              <button
                className="preview-button"
                onClick={() => onCopy(selectedItem.content, selectedItem.content_type)}
                title="Copy to clipboard"
              >
                📋 Copy
              </button>
              {selectedItem.content_type.startsWith('text') && (
                <button
                  className="preview-button edit"
                  onClick={handleEdit}
                  title="Edit content (Ctrl+E)"
                >
                  ✏️ Edit
                </button>
              )}
            </>
          )}
        </div>
      </div>
      <div className="preview-content">
        {isEditing ? (
          <textarea
            ref={textareaRef}
            className="preview-edit-textarea"
            value={editedContent}
            onChange={e => setEditedContent(e.target.value)}
            placeholder="Edit your content here..."
            spellCheck={false}
          />
        ) : selectedItem.content_type.startsWith('image/') ? (
          <div className="preview-image-container">
            <img
              ref={previewRef as React.RefObject<HTMLImageElement>}
              src={`data:${selectedItem.content_type};base64,${selectedItem.content}`}
              alt="Clipboard image"
              className="preview-image"
            />
          </div>
        ) : (
          <pre className="preview-code" onClick={() => setIsEditing(true)}>
            <code ref={previewRef} className={`language-${language}`}>
              {selectedItem.content}
            </code>
          </pre>
        )}
      </div>
    </div>
  );
};


function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [password, setPassword] = useState("");
  const [copiedToast, setCopiedToast] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);

  const checkVaultStatus = async () => {
    try {
      const isUnlocked = await invoke<boolean>("check_vault_status");

      if (!isUnlocked) {
        setShowPasswordPrompt(true);
        setResults([]);
      }

      return isUnlocked;
    } catch (error) {
      console.error("Failed to check vault status:", error);
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

      if (error && error.toString().includes("not unlocked")) {
        setShowPasswordPrompt(true);
      }

      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  const copyToClipboard = async (content: string, contentType?: string) => {
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
      // show toast
      setCopiedToast(true);
      setTimeout(() => setCopiedToast(false), 1300);
    } catch (error) {
      console.error("Copy failed:", error);
    }
  };


  const handleUnlock = async () => {
    try {
      const success = await invoke<boolean>("unlock_vault", { password });
      if (success) {
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
          copyToClipboard(results[selectedIndex].content, results[selectedIndex].content_type);
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
        behavior: "instant",
        block: "nearest",
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

  useEffect(() => {
    if (showPasswordPrompt) return;

    let unlisten: (() => void) | undefined;

    const setupEventListener = async () => {
      try {
        unlisten = await listen("clipboard-updated", () => {
          console.log("Clipboard updated, refreshing results...");
          searchClipboard(query);
        });
      } catch (error) {
        console.error("Failed to setup clipboard event listener:", error);
      }
    };

    setupEventListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [query, showPasswordPrompt]);


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

      <div className="main-content">
        <div className="results-panel">
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
                    onClick={() => setSelectedIndex(index)}
                  >
                    <div className="result-content">
                      {result.content_type.startsWith('image/') ? (
                        <div className="image-result">
                          <img
                            src={`data:${result.content_type};base64,${result.content}`}
                            alt="Clipboard image"
                            className="result-image-thumbnail"
                          />
                          <div className="image-info">
                            Image ({Math.round(result.content.length * 0.75 / 1024)} KB)
                          </div>
                        </div>
                      ) : (
                        highlightText(getWindowedContent(result.content, query), query)
                      )}
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
        </div>

        <PreviewPane
          selectedItem={results[selectedIndex] || null}
          onCopy={copyToClipboard}
        />
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

      {copiedToast && <div className="copy-notification">Copied!</div>}
    </div>
  );
}

export default App;
