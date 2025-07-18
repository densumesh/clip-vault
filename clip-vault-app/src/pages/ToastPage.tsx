import React, { useEffect, useState } from 'react';
import { ToastNotification } from '../components/ToastNotification';

interface ToastData {
  message: string;
  duration?: number;
}

const ToastPage: React.FC = () => {
  const [toastData, setToastData] = useState<ToastData | null>(null);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    // Show toast immediately when page loads
    setToastData({ message: 'Copied to clipboard' });
    setIsVisible(true);
  }, []);

  return (
    <div className="toast-page">
      {toastData && (
        <ToastNotification
          message={toastData.message}
          isVisible={isVisible}
          duration={toastData.duration}
        />
      )}
    </div>
  );
};

export default ToastPage;