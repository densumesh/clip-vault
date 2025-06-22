import React, { useRef, useEffect, memo, useMemo } from "react";
import type { ResultsListProps } from "../types";

// Text processing cache for performance
const TEXT_PROCESSING_CACHE = new Map<string, any>();
const CACHE_SIZE_LIMIT = 200;

export const ResultsList: React.FC<ResultsListProps> = ({
  results,
  selectedIndex,
  query,
  onSelect,
  loading,
  formatTimestamp,
  getWindowedContent,
  highlightText,
}) => {
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    const activeEl = resultRefs.current[selectedIndex];
    if (activeEl) {
      // Use a slight delay to ensure smooth scrolling feels natural
      requestAnimationFrame(() => {
        activeEl.scrollIntoView({
          behavior: "smooth",
          block: "center",
          inline: "nearest"
        });
      });
    }
  }, [selectedIndex]);

  const Row = memo(({ result, index, isSelected }: { result: any, index: number, isSelected: boolean }) => {
    const processedContent = useMemo(() => {
      if (result.content_type.startsWith('image/')) {
        return {
          type: 'image',
          content: result.content,
          contentType: result.content_type,
          size: Math.round(result.content.length * 0.75 / 1024)
        };
      }

      const cacheKey = `${result.id}-${query}`;
      const cached = TEXT_PROCESSING_CACHE.get(cacheKey);
      if (cached) {
        return cached;
      }

      const windowedContent = getWindowedContent(result.content, query);
      const highlighted = highlightText(windowedContent, query);
      
      const processed = {
        type: 'text',
        content: highlighted
      };

      // Cache management
      if (TEXT_PROCESSING_CACHE.size >= CACHE_SIZE_LIMIT) {
        const firstKey = TEXT_PROCESSING_CACHE.keys().next().value;
        if (firstKey) {
          TEXT_PROCESSING_CACHE.delete(firstKey);
        }
      }
      TEXT_PROCESSING_CACHE.set(cacheKey, processed);
      
      return processed;
    }, [result.content, result.content_type, result.id, query]);

    return (
      <div
        ref={el => {
          if (el) resultRefs.current[index] = el;
        }}
        className={`result-item ${isSelected ? "selected" : ""}`}
        onClick={() => onSelect(index)}
      >
        <div className="result-content">
          {processedContent.type === 'image' ? (
            <div className="image-result">
              <img
                src={`data:${processedContent.contentType};base64,${processedContent.content}`}
                alt="Clipboard image"
                className="result-image-thumbnail"
                loading="lazy"
                style={{ imageRendering: 'auto' }}
              />
              <div className="image-info">Image ({processedContent.size} KB)</div>
            </div>
          ) : (
            processedContent.content
          )}
        </div>
        <div className="result-meta">
          <span className="result-time">{formatTimestamp(result.timestamp)}</span>
          <span className="result-type">{result.content_type}</span>
        </div>
      </div>
    );
  });

  return (
    <div className="results-container">
      {results.length === 0 ? (
        <div className="empty-state">
          {query ? "No matches found" : "No clipboard history yet"}
        </div>
      ) : (
        <div className="results-list">
          {results.map((result, idx) => (
            <Row
              key={result.id}
              result={result}
              index={idx}
              isSelected={idx === selectedIndex}
            />
          ))}
        </div>
      )}

      {loading && <div className="loading-overlay"><span>Searching...</span></div>}
    </div>
  );
};