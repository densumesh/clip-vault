import React, { useRef, useEffect, memo } from "react";
import type { ResultsListProps } from "../types";

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
      activeEl.scrollIntoView({
        behavior: "instant",
        block: "nearest",
      });
    }
  }, [selectedIndex]);

  const Row = memo(({ index }: { index: number }) => {
    const result = results[index];
    return (
      <div
        ref={el => {
          if (el) resultRefs.current[index] = el;
        }}
        className={`result-item ${index === selectedIndex ? "selected" : ""}`}
        onClick={() => onSelect(index)}
      >
        <div className="result-content">
          {result.content_type.startsWith('image/') ? (
            <div className="image-result">
              <img
                src={`data:${result.content_type};base64,${result.content}`}
                alt="Clipboard image"
                className="result-image-thumbnail"
              />
              <div className="image-info">Image ({Math.round(result.content.length * 0.75 / 1024)} KB)</div>
            </div>
          ) : (
            highlightText(getWindowedContent(result.content, query), query)
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
          {results.map((_, idx) => (
            <Row key={results[idx].id} index={idx} />
          ))}
        </div>
      )}

      {loading && <div className="loading-overlay"><span>Searching...</span></div>}
    </div>
  );
};