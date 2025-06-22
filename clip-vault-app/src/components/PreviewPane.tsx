import React, { useRef, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import hljs from 'highlight.js';
import type { PreviewPaneProps } from "../types";
import { getContentStats } from "../utils/textUtils";

export const PreviewPane: React.FC<PreviewPaneProps> = ({ selectedItem, onCopy }) => {
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

  useEffect(() => {
    const highlightContent = async () => {
      setIsEditing(false);
      setEditedContent(selectedItem?.content || "");
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