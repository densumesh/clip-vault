import React from "react";
import type { PasswordPromptProps } from "../types";

export const PasswordPrompt: React.FC<PasswordPromptProps> = ({
  isVisible,
  password,
  onPasswordChange,
  onUnlock,
  onCancel,
}) => {
  if (!isVisible) return null;

  return (
    <div className="modal-overlay">
      <div className="modal">
        <div className="modal-header">
          <h2>Unlock Vault</h2>
        </div>
        <div className="modal-content">
          <p>Enter your vault password to unlock the database:</p>
          <input
            type="password"
            value={password}
            onChange={(e) => onPasswordChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                onUnlock();
              } else if (e.key === "Escape") {
                onCancel();
              }
            }}
            placeholder="Vault password"
            autoFocus
          />
        </div>
        <div className="modal-footer">
          <button
            className="modal-button secondary"
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            className="modal-button primary"
            onClick={onUnlock}
          >
            Unlock
          </button>
        </div>
      </div>
    </div>
  );
};