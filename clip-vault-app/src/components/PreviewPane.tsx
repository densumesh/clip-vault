import React, { useRef, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PreviewPaneProps } from "../types";
import { getContentStats } from "../utils/textUtils";


export const PreviewPane: React.FC<PreviewPaneProps> = ({ selectedItem, onCopy }) => {
  const previewRef = useRef<HTMLElement>(null);
  const editorRef = useRef<any>(null);
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

      // Update the selected item content
      if (selectedItem) {
        selectedItem.content = editedContent;
        // Update the preview with plain text
        if (previewRef.current) {
          setTimeout(() => {
            if (previewRef.current) {
              previewRef.current.innerText = editedContent;
            }
          }, 0);
        }
      }
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
    setTimeout(() => {
      if (previewRef.current && selectedItem) {
        previewRef.current.innerText = selectedItem.content;
      }
    }, 0);
  };

  const handleEdit = () => {
    setIsEditing(true);
    // Focus the editor's textarea after it's rendered
    setTimeout(() => {
      const textarea = document.querySelector('.preview-edit-textarea') as HTMLTextAreaElement;
      if (textarea) {
        textarea.focus();
        // Set cursor to end
        textarea.setSelectionRange(textarea.value.length, textarea.value.length);
      }
    }, 100);
  };



  useEffect(() => {
    const updatePreview = async () => {
      setIsEditing(false);
      setEditedContent(selectedItem?.content || "");
      if (!selectedItem || !previewRef.current) return;

      if (selectedItem.content_type.startsWith("image/")) {
        previewRef.current.innerHTML = "";
        return;
      }

      // Use requestAnimationFrame to avoid blocking the UI
      requestAnimationFrame(() => {
        if (!previewRef.current) return;
        previewRef.current.innerText = selectedItem.content;
      });
    };

    updatePreview();
  }, [selectedItem]);

  // Handle keyboard shortcuts in edit mode
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if ((e.key === 's' || e.key === 'Enter') && isEditing) {
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
          {!selectedItem.content_type.startsWith("image/") ? (
            <div className="preview-stats">
              <span className="stat-item">{stats.lines} lines</span>
              <span className="stat-item">{stats.chars} chars</span>
              <span className="stat-item">{stats.words} words</span>
            </div>
          ) : (
            <div className="preview-stats">
              <span className="stat-item">{(() => {
                const img = new Image();
                img.src = `data:${selectedItem.content_type};base64,${selectedItem.content}`;
                return `${img.width} × ${img.height}px`;
              })()}</span>
              <span className="stat-item">Image</span>
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
                {isSaving ? "Saving..." : "Save"}
              </button>
              <button
                className="preview-button cancel"
                onClick={handleCancel}
                disabled={isSaving}
                title="Cancel editing (Esc)"
              >
                Cancel
              </button>
            </>
          ) : (
            <>
              <button
                className="preview-button"
                onClick={() => onCopy(selectedItem.content, selectedItem.content_type)}
                title="Copy to clipboard"
              >
                Copy
              </button>
              {selectedItem.content_type.startsWith('text') && (
                <button
                  className="preview-button edit"
                  onClick={handleEdit}
                  title="Edit content (Ctrl+E)"
                >
                  Edit
                </button>
              )}
            </>
          )}
        </div>
      </div>
      <div className="preview-content">
        {isEditing ? (
          <textarea
            ref={editorRef}
            value={editedContent}
            onChange={(e) => setEditedContent(e.target.value)}
            className="preview-edit-textarea"
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.stopPropagation();
                handleCancel();
              }
              if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
                e.stopPropagation();
              }
              if (e.key === 'Enter') {
                e.stopPropagation();
              }
            }}
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
            <code ref={previewRef}>
              {selectedItem.content}
            </code>
          </pre>
        )}
      </div>
    </div>
  );
};