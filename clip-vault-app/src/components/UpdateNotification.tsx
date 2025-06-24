import React, { useState, useEffect } from 'react';
import { UpdateService } from '../services/updateService';

interface UpdateNotificationProps {
  version: string;
  onClose: () => void;
}

export const UpdateNotification: React.FC<UpdateNotificationProps> = ({ version: updateVersion, onClose }) => {
  const [isInstalling, setIsInstalling] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Listen for update progress & install events
    const setupListeners = async () => {
      const progressUnlisten = await UpdateService.listenToUpdateProgress((progress) => {
        setDownloadProgress(progress);
      });

      const installedUnlisten = await UpdateService.listenToUpdateInstalled(() => {
        setIsInstalling(false);
        onClose();
      });

      return () => {
        progressUnlisten();
        installedUnlisten();
      };
    };

    setupListeners();
  }, [onClose]);

  const handleInstallUpdate = async () => {
    if (!updateVersion) return;

    setIsInstalling(true);
    setError(null);
    setDownloadProgress(0);

    try {
      await UpdateService.installUpdate();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to install update');
      setIsInstalling(false);
    }
  };

  if (error) {
    return (
      <div className="update-notification error">
        <div className="update-content">
          <div className="update-icon">⚠️</div>
          <div className="update-text">
            <h3>Update Error</h3>
            <p>{error}</p>
          </div>
        </div>
        <button className="update-button secondary" onClick={onClose}>
          Close
        </button>
      </div>
    );
  }

  if (isInstalling) {
    return (
      <div className="update-notification installing">
        <div className="update-content">
          <div className="update-icon">⬇️</div>
          <div className="update-text">
            <h3>Installing Update</h3>
            <p>Downloading version {updateVersion}...</p>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${downloadProgress}%` }}
              />
            </div>
            <p className="progress-text">{Math.round(downloadProgress)}%</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="update-notification available">
      <div className="update-content">
        <div className="update-icon">🔄</div>
        <div className="update-text">
          <h3>Update Available</h3>
          <p>Version {updateVersion} is ready to install.</p>
        </div>
      </div>
      <div className="update-buttons">
        <button className="update-button secondary" onClick={onClose}>
          Later
        </button>
        <button className="update-button primary" onClick={handleInstallUpdate}>
          Install Now
        </button>
      </div>
    </div>
  );
};