import React, { useEffect, useRef } from 'react';

interface ToastNotificationProps {
  message: string;
  isVisible: boolean;
  duration?: number;
}

export const ToastNotification: React.FC<ToastNotificationProps> = ({
  message,
  isVisible,
  duration = 2500
}) => {
  const toastRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (isVisible) {
      const timer = setTimeout(() => {
        if (toastRef.current) {
          toastRef.current.classList.add('hiding');
        }
      }, 300);

      return () => clearTimeout(timer);
    }
  }, [isVisible, duration]);

  if (!isVisible) return null;

  return (
    <div className="toast-notification" ref={toastRef}>
      <div className="toast-content">
        <div className="toast-icon">✅</div>
        <div className="toast-message">{message}</div>
      </div>
    </div>
  );
};