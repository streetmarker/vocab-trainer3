// src/notification.tsx — Entry point for the task-notification window
//
// Architecture mirrors popup.tsx exactly:
// - Window is created on-demand (first call) then PARKED at x=-2000,y=-2000
//   (NOT hidden — parking keeps JS alive, hide() throttles WebView2)
// - Rust calls emit_to("task-notification", "task-notification", payload)
// - We use getCurrentWebviewWindow().listen() NOT the global listen()
//   Reason: emit_to("task-notification", ...) targets this specific WebviewWindow;
//   global listen() registers as AnyLabel which does NOT match (Tauri 2 bug #11561)
//   getCurrentWebviewWindow().listen() registers correctly for this window's label

import React, { useEffect, useState, useCallback } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { TaskNotification } from "./components/TaskNotification/TaskNotification";
import "./styles/notification.css";

const appWindow = getCurrentWebviewWindow();

type NotifPayload = {
  termPl:      string;
  termEn:      string;
  partOfSpeech?: string;
  wordId:      number;
};

function NotificationApp() {
  const [payload, setPayload] = useState<NotifPayload | null>(null);

  const dismiss = useCallback(() => {
    setPayload(null);
  }, []);

  useEffect(() => {
    const unlisten = appWindow.listen<NotifPayload>(
      "task-notification",
      (event) => {
        // Reset first so TaskNotification remounts cleanly for the same word
        setPayload(null);
        requestAnimationFrame(() => setPayload(event.payload));
      }
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  if (!payload) {
    // Empty transparent placeholder while waiting for the event
    return <div style={{ width: "100%", height: "100%", background: "transparent" }} />;
  }

  return (
    <TaskNotification
      key={`${payload.wordId}-${Date.now()}`}
      termPl={payload.termPl}
      termEn={payload.termEn}
      partOfSpeech={payload.partOfSpeech}
      wordId={payload.wordId}
      onDismiss={dismiss}
    />
  );
}

ReactDOM.createRoot(document.getElementById("notification-root")!).render(
  <React.StrictMode>
    <NotificationApp />
  </React.StrictMode>
);
