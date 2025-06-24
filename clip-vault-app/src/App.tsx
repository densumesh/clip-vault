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
  OnboardingFlow,
  UpdateNotification,
} from "./components";

// Hooks
import { useClipboardSearch } from "./hooks/useClipboardSearch";
import { useVault } from "./hooks/useVault";
import { useKeyboardNavigation } from "./hooks/useKeyboardNavigation";
import { useClipboardUpdates } from "./hooks/useClipboardUpdates";
import { UpdateService } from "./services/updateService";

// Utils
import { formatTimestamp, getWindowedContent, highlightText } from "./utils/textUtils";

function App() {
  const [searching, setSearching] = useState(true);
  const [justCopied, setJustCopied] = useState(false);
  // Custom hooks
  const {
    query,
    setQuery,
    results,
    selectedIndex,
    setSelectedIndex,
    loading,
    loadingMore,
    hasMore,
    searchClipboard,
    loadMore,
    copyToClipboard,
  } = useClipboardSearch();

  const {
    isUnlocked,
    showPasswordPrompt,
    showOnboarding,
    password,
    setPassword,
    checkVaultStatus,
    handleUnlock,
    handleCancel,
    handleOnboardingComplete,
  } = useVault();

  // Handle copy with toast notification
  const handleCopy = async (content: string, contentType: string) => {
    setJustCopied(true);
    await copyToClipboard(content, contentType);
    const window = getCurrentWebviewWindow();
    window.hide();
  };

  // Keyboard navigation
  useKeyboardNavigation({
    results,
    selectedIndex,
    setSelectedIndex,
    onCopy: handleCopy,
    showPasswordPrompt,
    setSearching,
  });

  // Clipboard updates
  useClipboardUpdates({
    onClipboardUpdate: () => {
      // Don't refresh if we just copied something from the app
      if (!justCopied) {
        searchClipboard(query);
      }
    },
    showPasswordPrompt,
  });

  // Update notification state
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [showUpdateNotification, setShowUpdateNotification] = useState(false);

  useEffect(() => {
    // Check for updates once on app load
    const checkUpdates = async () => {
      try {
        const version = await UpdateService.checkForUpdates();
        if (version) {
          setUpdateVersion(version);
          setShowUpdateNotification(true);
        }
      } catch (e) {
        console.error("Failed to check for updates", e);
      }
    };

    checkUpdates();
  }, []);

  const handleCloseUpdateNotification = () => {
    setShowUpdateNotification(false);
  };

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
        searching={searching}
        setSearching={setSearching}
      />

      <div className="main-content">
        <div className="results-panel">
          <ResultsList
            results={results}
            selectedIndex={selectedIndex}
            query={query}
            onSelect={setSelectedIndex}
            loading={loading}
            loadingMore={loadingMore}
            hasMore={hasMore}
            onLoadMore={loadMore}
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

      <OnboardingFlow
        isVisible={showOnboarding}
        onComplete={handleOnboardingComplete}
      />

      {showUpdateNotification && updateVersion && (
        <UpdateNotification version={updateVersion} onClose={handleCloseUpdateNotification} />
      )}

    </div>
  );
}

export default App;