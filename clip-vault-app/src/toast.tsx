import React from "react";
import ReactDOM from "react-dom/client";
import ToastPage from "./pages/ToastPage";
import "./App.css";

ReactDOM.createRoot(document.getElementById("toast-root") as HTMLElement).render(
  <React.StrictMode>
    <ToastPage />
  </React.StrictMode>
);