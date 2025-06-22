import { useState, useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import 'highlight.js/styles/github-dark.css';
import "./App.css";

// Components
import {
  SearchInput,
  ResultsList,
  PreviewPane,
  PasswordPrompt,
  CopyNotification,
} from "./components";

// Hooks
import { useClipboardSearch } from "./hooks/useClipboardSearch";
import { useVault } from "./hooks/useVault";
import { useKeyboardNavigation } from "./hooks/useKeyboardNavigation";
import { useClipboardUpdates } from "./hooks/useClipboardUpdates";

// Utils
import { formatTimestamp, getWindowedContent, highlightText } from "./utils/textUtils";

function App() {
  const [copiedToast, setCopiedToast] = useState(false);

  // Custom hooks
  const {
    query,
    setQuery,
    results,
    selectedIndex,
    setSelectedIndex,
    loading,
    searchClipboard,
    copyToClipboard,
  } = useClipboardSearch();

  const {
    isUnlocked,
    showPasswordPrompt,
    password,
    setPassword,
    checkVaultStatus,
    handleUnlock,
    handleCancel,
  } = useVault();

  // Handle copy with toast notification
  const handleCopy = async (content: string, contentType: string) => {
    const success = await copyToClipboard(content, contentType);
    if (success) {
      setCopiedToast(true);
      setTimeout(() => setCopiedToast(false), 1300);
    }
  };

  // Keyboard navigation
  useKeyboardNavigation({
    results,
    selectedIndex,
    setSelectedIndex,
    onCopy: handleCopy,
    showPasswordPrompt,
  });

  // Clipboard updates
  useClipboardUpdates({
    onClipboardUpdate: () => searchClipboard(query),
    showPasswordPrompt,
  });

  // Window management
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
  }, [checkVaultStatus]);

  // Initialize app
  useEffect(() => {
    const initializeApp = async () => {
      if (isUnlocked) {
        searchClipboard("");
      }
    };

    initializeApp();
  }, [isUnlocked, searchClipboard]);

  return (
    <div className="app">
      <SearchInput
        query={query}
        onQueryChange={setQuery}
        loading={loading}
        resultsCount={results.length}
      />

      <div className="main-content">
        <div className="results-panel">
          <ResultsList
            results={results}
            selectedIndex={selectedIndex}
            query={query}
            onSelect={setSelectedIndex}
            loading={loading}
            formatTimestamp={formatTimestamp}
            getWindowedContent={getWindowedContent}
            highlightText={highlightText}
          />
        </div>

        <PreviewPane
          selectedItem={results[selectedIndex] || null}
          onCopy={handleCopy}
        />
      </div>

      <div className="help-text">
        Use ↑↓ to navigate • Enter to copy • Esc to close
      </div>

      <PasswordPrompt
        isVisible={showPasswordPrompt}
        password={password}
        onPasswordChange={setPassword}
        onUnlock={handleUnlock}
        onCancel={handleCancel}
      />

      <CopyNotification isVisible={copiedToast} />
    </div>
  );
}

export default App;