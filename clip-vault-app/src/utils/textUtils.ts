import React from "react";

export const formatTimestamp = (timestamp: number): string => {
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

export const getWindowedContent = (content: string, query: string, maxLength: number = 100): string => {
  if (!query.trim()) {
    // No search query, just truncate normally
    if (content.length <= maxLength) return content;
    return content.substring(0, maxLength) + "...";
  }

  // Find the first match
  const regex = new RegExp(query.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&'), 'gi');
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

export const highlightText = (text: string, query: string): React.ReactNode => {
  if (!query.trim()) return text;

  const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&')})`, 'gi');
  const parts = text.split(regex);

  return parts.map((part, index) =>
    regex.test(part) ? (
      React.createElement('mark', { key: index, className: 'highlight' }, part)
    ) : (
      part
    )
  );
};

export const getContentStats = (content: string) => {
  const lines = content.split('\n').length;
  const chars = content.length;
  const words = content.trim().split(/\s+/).length;
  return { lines, chars, words };
};