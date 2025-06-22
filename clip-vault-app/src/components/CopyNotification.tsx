import React from "react";
import type { CopyNotificationProps } from "../types";

export const CopyNotification: React.FC<CopyNotificationProps> = ({ isVisible }) => {
  if (!isVisible) return null;

  return <div className="copy-notification">Copied!</div>;
};