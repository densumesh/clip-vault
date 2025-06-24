import React, {
  useRef,
  useMemo,
  useLayoutEffect,
  useEffect,
  memo,
} from "react";
import type { ResultsListProps } from "../types";

const TEXT_PROCESSING_CACHE = new Map<string, any>();
const CACHE_SIZE_LIMIT = 200;

interface RowProps {
  result: any;
  index: number;
  isSelected: boolean;
  query: string;
  onSelect: (idx: number) => void;
  getWindowedContent: (content: string, q: string) => string;
  highlightText: (content: string, q: string) => React.ReactNode;
  formatTimestamp: (ts: number) => string;
}

const Row = memo(React.forwardRef<HTMLDivElement, RowProps>(
  ({
    result,
    index,
    isSelected,
    query,
    onSelect,
    getWindowedContent,
    highlightText,
    formatTimestamp,
  }, ref) => {
    const processedContent = useMemo(() => {
      if (result.content_type.startsWith("image/")) {
        return {
          type: "image",
          content: result.content,
          contentType: result.content_type,
          size: Math.round((result.content.length * 0.75) / 1024),
        } as const;
      }

      const cacheKey = `${result.id}-${query}`;
      const cached = TEXT_PROCESSING_CACHE.get(cacheKey);
      if (cached) return cached;

      const windowedContent = getWindowedContent(result.content, query);
      const highlighted = highlightText(windowedContent, query);

      const processed = { type: "text", content: highlighted } as const;

      if (TEXT_PROCESSING_CACHE.size >= CACHE_SIZE_LIMIT) {
        const firstKey = TEXT_PROCESSING_CACHE.keys().next().value;
        if (firstKey) TEXT_PROCESSING_CACHE.delete(firstKey);
      }
      TEXT_PROCESSING_CACHE.set(cacheKey, processed);
      return processed;
    }, [result, query, getWindowedContent, highlightText]);

    return (
      <div
        className={`result-item ${isSelected ? "selected" : ""}`}
        onClick={() => onSelect(index)}
        ref={ref}
      >
        <div className="result-content">
          {processedContent.type === "image" ? (
            <div className="image-result">
              <img
                src={`data:${processedContent.contentType};base64,${processedContent.content}`}
                alt="Clipboard preview"
                className="result-image-thumbnail"
                loading="lazy"
                draggable={false}
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
  }
));

export const ResultsList: React.FC<ResultsListProps> = ({
  results,
  selectedIndex,
  query,
  onSelect,
  loading,
  loadingMore,
  hasMore,
  onLoadMore,
  formatTimestamp,
  getWindowedContent,
  highlightText,
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Clean up refs when results change to prevent stale references
  useEffect(() => {
    resultRefs.current = resultRefs.current.slice(0, results.length);
  }, [results.length]);

  useEffect(() => {
    if (!hasMore || loadingMore || results.length < 5) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry.isIntersecting) {
          onLoadMore();
        }
      },
      {
        threshold: 0.1,
        rootMargin: '100px', // Start loading when element is 100px from viewport
      }
    );

    // Find the 5th element from the end
    const triggerIndex = Math.max(0, results.length - 5);
    const triggerElement = resultRefs.current[triggerIndex];

    if (triggerElement) {
      observer.observe(triggerElement);
    }

    return () => {
      observer.disconnect();
    };
  }, [hasMore, loadingMore, onLoadMore, results.length]);

  useEffect(() => {
    if (results.length > 0 && selectedIndex === 0) {
      const el = resultRefs.current[0];
      if (el) {
        el.scrollIntoView({ behavior: "instant", block: "nearest" });
      }
    }
  }, [results, selectedIndex]);

  useLayoutEffect(() => {
    const el = resultRefs.current[selectedIndex];
    const container = containerRef.current;
    if (el && container) {

      // trigger smooth centering
      requestAnimationFrame(() => {
        el.scrollIntoView({ behavior: "instant", block: "nearest" });
      });
    }
  }, [selectedIndex]);

  return (
    <div className={`results-container`} ref={containerRef}>
      {results.length === 0 ? (
        <div className="empty-state">
          {query ? "No matches found" : "No clipboard history yet"}
        </div>
      ) : (
        <div className="results-list">
          {results.map((result, idx) => (
            <Row
              ref={(el) => {
                if (el) resultRefs.current[idx] = el;
              }}
              key={result.id}
              result={result}
              index={idx}
              isSelected={idx === selectedIndex}
              query={query}
              onSelect={onSelect}
              getWindowedContent={getWindowedContent}
              highlightText={highlightText}
              formatTimestamp={formatTimestamp}
            />
          ))}

          {/* Loading indicator at the bottom */}
          {loadingMore && (
            <div
              className="loading-indicator"
              style={{
                height: '20px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center'
              }}
            >
              <span>Loading more...</span>
            </div>
          )}
        </div>
      )}

      {loading && query.length > 0 && (
        <div className="loading-overlay">
          <span>Searching...</span>
        </div>
      )}
    </div>
  );
};