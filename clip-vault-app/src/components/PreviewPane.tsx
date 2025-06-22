import React, { useRef, useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import hljs from 'highlight.js';
import type { PreviewPaneProps } from "../types";
import { getContentStats } from "../utils/textUtils";

// Constants for performance optimization
const MAX_HIGHLIGHT_SIZE = 50000; // 50KB limit for syntax highlighting
const HIGHLIGHT_CACHE = new Map<string, { value: string; language: string }>();
const CACHE_SIZE_LIMIT = 100;

// Smart language detection without hljs.highlightAuto
const LANGUAGE_DETECTORS = {
  json: {
    patterns: [
      /^\s*[{\[]/, // Starts with { or [
      /"\w+"\s*:\s*/, // Key-value pairs
      /".*"\s*[,}\]]/, // JSON string values
      /^\s*[}\]]\s*$/ // Ends with } or ]
    ],
    minMatches: 2,
    keywords: ['null', 'true', 'false']
  },
  javascript: {
    patterns: [
      /\b(function|const|let|var|class|import|export)\b/,
      /\(.*\)\s*=>/, // Arrow functions
      /console\.(log|error|warn)/,
      /\{[\s\S]*\}/, // Code blocks
      /\b(if|else|for|while|return)\b/
    ],
    minMatches: 2,
    keywords: ['function', 'const', 'let', 'var', 'return', 'if', 'else']
  },
  python: {
    patterns: [
      /\b(def|class|import|from|if __name__)\b/,
      /^\s*#.*$/, // Comments
      /:\s*$/, // Colons at end of line
      /\b(True|False|None|print)\b/,
      /^\s{4}|^\t/ // Indentation
    ],
    minMatches: 2,
    keywords: ['def', 'class', 'import', 'from', 'True', 'False', 'None']
  },
  html: {
    patterns: [
      /<\/?html[^>]*>/i,
      /<\/?head[^>]*>/i,
      /<\/?body[^>]*>/i,
      /<\w+[^>]*>/,
      /<\/\w+>/
    ],
    minMatches: 2,
    keywords: ['html', 'head', 'body', 'div', 'span']
  },
  css: {
    patterns: [
      /[\w-]+\s*:\s*[^;]+;/,
      /\{[^}]*\}/,
      /@\w+/,
      /\/\*[\s\S]*?\*\//,
      /\.[\w-]+\s*\{/
    ],
    minMatches: 2,
    keywords: ['color', 'background', 'margin', 'padding', 'display']
  },
  xml: {
    patterns: [
      /<\?xml[^>]*>/,
      /<\w+[^>]*\/>/, // Self-closing tags
      /<\w+[^>]*>[\s\S]*?<\/\w+>/, // Tag pairs
      /&\w+;/ // Entities
    ],
    minMatches: 2,
    keywords: ['xml', 'version', 'encoding']
  },
  sql: {
    patterns: [
      /\b(SELECT|FROM|WHERE|JOIN|INSERT|UPDATE|DELETE)\b/i,
      /\b(CREATE|ALTER|DROP)\s+(TABLE|DATABASE|INDEX)\b/i,
      /\b(AND|OR|NOT|IN|LIKE)\b/i,
      /;\s*$/
    ],
    minMatches: 2,
    keywords: ['SELECT', 'FROM', 'WHERE', 'INSERT', 'UPDATE', 'DELETE']
  },
  bash: {
    patterns: [
      /^#!/,
      /\$\w+/,
      /\|\s*\w+/,
      /&&|\|\|/,
      /\b(echo|cd|ls|grep|find)\b/
    ],
    minMatches: 2,
    keywords: ['echo', 'cd', 'ls', 'grep', 'find', 'chmod']
  },
  yaml: {
    patterns: [
      /^\s*\w+:\s*/,
      /^\s*-\s+/,
      /---\s*$/,
      /^\s*#/
    ],
    minMatches: 2,
    keywords: ['version', 'name', 'description']
  },
  markdown: {
    patterns: [
      /^#+\s+/, // Headers
      /\*\*.*\*\*/, // Bold
      /\*.*\*/, // Italic
      /^\s*[*-]\s+/, // Lists
      /\[.*\]\(.*\)/ // Links
    ],
    minMatches: 2,
    keywords: []
  }
};

// Plain text indicators - if these match, it's likely plain text
const PLAIN_TEXT_INDICATORS = [
  /^[\w\s.,;:!?()\[\]{}"'-]+$/, // Only basic punctuation
  /^\d+[\s\w]*$/, // Numbers with simple text
  /^[A-Z][a-z\s,]+[.!?]$/, // Simple sentences
  /^\w+\s+\w+\s+\w+$/, // Three words
];

const detectLanguage = (content: string): string => {
  const trimmed = content.trim();
  
  // Skip detection for very short content
  if (trimmed.length < 10) {
    return 'plaintext';
  }
  
  // Check if it looks like plain text first
  const isPlainText = PLAIN_TEXT_INDICATORS.some(pattern => 
    pattern.test(trimmed.slice(0, 200)) // Check first 200 chars
  );
  
  if (isPlainText) {
    return 'plaintext';
  }
  
  // Score each language
  const scores: Record<string, number> = {};
  
  for (const [lang, detector] of Object.entries(LANGUAGE_DETECTORS)) {
    let score = 0;
    
    // Pattern matching
    const patternMatches = detector.patterns.filter(pattern => 
      pattern.test(content)
    ).length;
    
    if (patternMatches >= detector.minMatches) {
      score += patternMatches * 2;
    }
    
    // Keyword matching
    const keywordMatches = detector.keywords.filter(keyword => 
      new RegExp(`\\b${keyword}\\b`, 'i').test(content)
    ).length;
    
    score += keywordMatches;
    
    if (score > 0) {
      scores[lang] = score;
    }
  }
  
  // Find the highest scoring language
  const bestMatch = Object.entries(scores).reduce(
    (best, [lang, score]) => score > best.score ? { lang, score } : best,
    { lang: 'plaintext', score: 0 }
  );
  
  // Only use language detection if we have a confident match
  return bestMatch.score >= 3 ? bestMatch.lang : 'plaintext';
};

export const PreviewPane: React.FC<PreviewPaneProps> = ({ selectedItem, onCopy }) => {
  const previewRef = useRef<HTMLElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [language, setLanguage] = useState<string>("");
  const [isEditing, setIsEditing] = useState(false);
  const [editedContent, setEditedContent] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isHighlighting, setIsHighlighting] = useState(false);

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp / 1_000_000);
    return date.toLocaleString();
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

  // Optimized highlighting with caching and size limits
  const highlightedContent = useMemo(() => {
    if (!selectedItem || selectedItem.content_type.startsWith("image/")) {
      return { content: selectedItem?.content || "", language: "image", isHighlighted: false };
    }

    const content = selectedItem.content;
    const contentSize = new Blob([content]).size;

    // Skip highlighting for very large content
    if (contentSize > MAX_HIGHLIGHT_SIZE) {
      return { content, language: "plaintext", isHighlighted: false };
    }

    // Check cache first
    const cacheKey = content.slice(0, 1000); // Use first 1KB as cache key
    const cached = HIGHLIGHT_CACHE.get(cacheKey);
    if (cached) {
      return {
        content: cached.value,
        language: cached.language || "plaintext",
        isHighlighted: cached.language !== "plaintext"
      };
    }

    // Smart language detection
    const detectedLanguage = detectLanguage(content);
    const shouldHighlight = detectedLanguage !== 'plaintext';
    
    let highlightedValue = content;
    
    // Only perform syntax highlighting if we detected a specific language
    if (shouldHighlight) {
      try {
        const highlighted = hljs.highlight(content, { language: detectedLanguage });
        highlightedValue = highlighted.value;
      } catch (error) {
        // If specific language highlighting fails, fall back to plain text
        console.warn(`Highlighting failed for ${detectedLanguage}, using plain text:`, error);
        highlightedValue = content;
      }
    }

    // Cache the result
    if (HIGHLIGHT_CACHE.size >= CACHE_SIZE_LIMIT) {
      const firstKey = HIGHLIGHT_CACHE.keys().next().value;
      if (firstKey) {
        HIGHLIGHT_CACHE.delete(firstKey);
      }
    }
    HIGHLIGHT_CACHE.set(cacheKey, {
      value: highlightedValue,
      language: detectedLanguage
    });

    return {
      content: highlightedValue,
      language: detectedLanguage,
      isHighlighted: shouldHighlight
    };
  }, [selectedItem]);

  useEffect(() => {
    const updatePreview = async () => {
      setIsEditing(false);
      setEditedContent(selectedItem?.content || "");
      if (!selectedItem || !previewRef.current) return;

      if (selectedItem.content_type.startsWith("image/")) {
        setLanguage("image");
        previewRef.current.innerHTML = "";
        return;
      }

      setIsHighlighting(true);

      // Use requestAnimationFrame to avoid blocking the UI
      requestAnimationFrame(() => {
        if (!previewRef.current) return;

        if (highlightedContent.isHighlighted) {
          previewRef.current.innerHTML = highlightedContent.content;
        } else {
          previewRef.current.innerText = highlightedContent.content;
        }

        setLanguage(highlightedContent.language);
        setIsHighlighting(false);
      });
    };

    updatePreview();
  }, [selectedItem, highlightedContent]);

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
              {isHighlighting && <span className="stat-item">Analyzing...</span>}
              {stats.chars > MAX_HIGHLIGHT_SIZE && <span className="stat-item warning">Large file - highlighting disabled</span>}
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
          <pre className="preview-code">
            <code ref={previewRef} className={`language-${language}`}>
              {selectedItem.content}
            </code>
          </pre>
        )}
      </div>
    </div>
  );
};