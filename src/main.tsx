// src/main.tsx — Entry point for the main application window only.
// The task-notification window uses its own notification.html → notification.tsx.
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/global.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
