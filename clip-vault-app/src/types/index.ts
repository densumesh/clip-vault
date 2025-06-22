export interface SearchResult {
  id: string;
  content: string;
  timestamp: number;
  content_type: string;
}

export interface AppSettings {
  poll_interval_ms: number;
  vault_path: string;
  auto_lock_minutes: number;
}

export interface PreviewPaneProps {
  selectedItem: SearchResult | null;
  onCopy: (content: string, contentType: string) => void;
  onEdit?: () => void;
}

export interface SearchInputProps {
  query: string;
  onQueryChange: (query: string) => void;
  loading: boolean;
  resultsCount: number;
}

export interface ResultsListProps {
  results: SearchResult[];
  selectedIndex: number;
  query: string;
  onSelect: (index: number) => void;
  loading: boolean;
  formatTimestamp: (timestamp: number) => string;
  getWindowedContent: (content: string, query: string, maxLength?: number) => string;
  highlightText: (text: string, query: string) => React.ReactNode;
}

export interface PasswordPromptProps {
  isVisible: boolean;
  password: string;
  onPasswordChange: (password: string) => void;
  onUnlock: () => void;
  onCancel: () => void;
}

export interface CopyNotificationProps {
  isVisible: boolean;
}