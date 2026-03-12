// src/popup.tsx — Entry point for the floating popup window
//
// Architecture:
// - Window is always "visible" but parked at x=-2000,y=-2000 when idle
//   (prevents browser JS throttling that happens with hidden windows)
// - Rust calls emit_to("popup", "load_exercise", {wordId}) to trigger display
// - We use getCurrentWebviewWindow().listen() NOT listen() from @tauri-apps/api/event
//   Reason: global listen() registers as AnyLabel target, but emit_to("popup") targets
//   WebviewWindow{label:"popup"} — they don't match (Tauri 2 bug #11561)
//   getCurrentWebviewWindow().listen() registers correctly as WebviewWindow{label:"popup"}

import React, { useEffect, useState, useCallback } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ExercisePopup } from "./components/ExercisePopup/ExercisePopup";
import type { Exercise, AnswerResult } from "./types";
import { api } from "./hooks/useTauri";
import "./styles/global.css";
import "./styles/popup-window.css";

const appWindow = getCurrentWebviewWindow();

function PopupApp() {
  const [exercise, setExercise] = useState<Exercise | null>(null);

  const loadExercise = useCallback(async (wordId: number) => {
    try {
      const ex = await api.getExercise(wordId);
      setExercise(ex);
    } catch (err) {
      console.error("Błąd ładowania ćwiczenia:", err);
      await api.hidePopup();
    }
  }, []);

  useEffect(() => {
    // Listen on this specific window — matches emit_to("popup", "load_exercise")
    const unlisten = appWindow.listen<{ wordId: number }>(
      "load_exercise",
      (e) => {
        setExercise(null); // reset first in case same word shown again
        loadExercise(e.payload.wordId);
      }
    );
    return () => { unlisten.then((f) => f()); };
  }, [loadExercise]);

  const dismiss = async () => {
    setExercise(null);
    await api.hidePopup();
  };

  const complete = async (_result: AnswerResult) => {
    setExercise(null);
    await api.hidePopup();
  };

  if (!exercise) return <div className="popup-waiting" />;

  return (
    <ExercisePopup
      exercise={exercise}
      onComplete={complete}
      onDismiss={dismiss}
    />
  );
}

ReactDOM.createRoot(document.getElementById("popup-root")!).render(
  <React.StrictMode>
    <PopupApp />
  </React.StrictMode>
);
