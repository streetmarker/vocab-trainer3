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
  const [allMastered, setAllMastered] = useState(false);
  const [activeCat, setActiveCat] = useState<string | null>(null);

  const loadExercise = useCallback(async (wordId: number) => {
    try {
      const category = localStorage.getItem("active_category");
      const catFilter = category === "Wszystkie" ? null : category;
      setActiveCat(category);

      // Sprawdzamy czy dla danej kategorii mamy coś do roboty
      const nextWord = await api.getNextReviewWord(catFilter);
      
      if (!nextWord) {
        setAllMastered(true);
        return;
      }

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
        setExercise(null);
        setAllMastered(false);
        loadExercise(e.payload.wordId);
      }
    );
    return () => { unlisten.then((f) => f()); };
  }, [loadExercise]);

  const dismiss = async () => {
    setExercise(null);
    setAllMastered(false);
    await api.hidePopup();
  };

  const complete = async (_result: AnswerResult) => {
    setExercise(null);
    await api.hidePopup();
  };

  if (allMastered) {
    return (
      <div className="popup-root entering">
        <div className="popup-header">
          <div className="popup-logo">
            <span className="logo-icon">✨</span>
            <span className="logo-text">VocabTrainer</span>
          </div>
          <button className="popup-dismiss" onClick={dismiss}>✕</button>
        </div>
        <div className="popup-body">
          <div className="intro-view" style={{ textAlign: 'center', padding: '40px 20px' }}>
            <div style={{ fontSize: '48px', marginBottom: '20px' }}>🏆</div>
            <h2 style={{ color: 'white', marginBottom: '12px' }}>Świetna robota!</h2>
            <p style={{ color: 'var(--muted)', lineHeight: '1.6', fontSize: '15px' }}>
              Opanowałeś wszystkie zaplanowane słowa w kategorii <strong>{activeCat}</strong>.
            </p>
            <p style={{ color: 'var(--muted)', marginTop: '16px', fontSize: '14px' }}>
              Aby kontynuować naukę teraz, zmień kategorię w panelu głównym lub poczekaj na kolejne powtórki SRS.
            </p>
            <button className="btn-primary" style={{ marginTop: '32px', width: '100%' }} onClick={dismiss}>
              Zamknij
            </button>
          </div>
        </div>
      </div>
    );
  }

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
