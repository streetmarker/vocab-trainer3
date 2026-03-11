// src/App.tsx
import React, { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Dashboard } from "./components/Dashboard/Dashboard";
import { VocabManager } from "./components/VocabManager/VocabManager";
import { ExercisePopup } from "./components/ExercisePopup/ExercisePopup";
import type { AppRoute, Exercise, AnswerResult } from "./types";
import { api } from "./hooks/useTauri";
import "./styles/global.css";

export default function App() {
  const [route, setRoute] = useState<AppRoute>("dashboard");
  const [pendingExercise, setPendingExercise] = useState<Exercise | null>(null);
  const [showResult, setShowResult] = useState<AnswerResult | null>(null);

  // Listen for navigation events from tray menu
  useEffect(() => {
    const unlisten = listen<AppRoute>("navigate", (e) => {
      setRoute(e.payload);
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // Listen for scheduler-triggered exercises
  useEffect(() => {
    const unlisten = listen<{ wordId: number }>("show_exercise", async (e) => {
      try {
        const exercise = await api.getExercise(e.payload.wordId);
        setPendingExercise(exercise);
      } catch (err) {
        console.error("Błąd ładowania ćwiczenia:", err);
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // Listen for session start
  useEffect(() => {
    const unlisten = listen("session_started", async () => {
      try {
        const result = await api.startSession();
        if (result) {
          setPendingExercise(result.exercise);
        }
      } catch (err) {
        console.error("Błąd startu sesji:", err);
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleExerciseComplete = (result: AnswerResult) => {
    setShowResult(result);
    setPendingExercise(null);
    setTimeout(() => setShowResult(null), 2000);
  };

  const navItems: { id: AppRoute; icon: string; label: string }[] = [
    { id: "dashboard", icon: "📊", label: "Panel główny" },
    { id: "vocab",     icon: "📚", label: "Słownictwo" },
    { id: "settings",  icon: "⚙️", label: "Ustawienia" },
  ];

  return (
    <div className="app-root">
      {/* ── Sidebar ───────────────────────────────────────────────────── */}
      <nav className="sidebar">
        <div className="sidebar-logo">
          <div className="logo-mark">V</div>
          <div>
            <div className="logo-name">VocabTrainer</div>
            <div className="logo-tagline">nauka angielskiego</div>
          </div>
        </div>

        <div className="nav-links">
          {navItems.map((item) => (
            <button
              key={item.id}
              className={`nav-link ${route === item.id ? "active" : ""}`}
              onClick={() => setRoute(item.id)}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
              {route === item.id && <div className="nav-active-bar" />}
            </button>
          ))}
        </div>

        <SchedulerStatus />
      </nav>

      {/* ── Main Content ──────────────────────────────────────────────── */}
      <main className="main-content">
        {route === "dashboard" && <Dashboard />}
        {route === "vocab" && <VocabManager />}
        {route === "settings" && <SettingsPage />}
      </main>

      {/* ── Inline Exercise Popup (when triggered) ────────────────────── */}
      {pendingExercise && (
        <div className="exercise-overlay">
          <ExercisePopup
            exercise={pendingExercise}
            onComplete={handleExerciseComplete}
            onDismiss={() => setPendingExercise(null)}
          />
        </div>
      )}

      {/* ── Toast result ──────────────────────────────────────────────── */}
      {showResult && (
        <div className={`result-toast ${showResult.wasCorrect ? "correct" : "incorrect"}`}>
          {showResult.wasCorrect
            ? `✓ Poprawnie!${showResult.streak > 1 ? ` 🔥 Seria: ${showResult.streak}` : ""}`
            : `Następna powtórka za ${(showResult.newIntervalDays * 24).toFixed(0)}h`}
        </div>
      )}
    </div>
  );
}

// ─── Scheduler Status Widget ──────────────────────────────────────────────────

const SchedulerStatus: React.FC = () => {
  const [status, setStatus] = useState<any>(null);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    const poll = async () => {
      try {
        const s = await api.getSchedulerStatus();
        setStatus(s);
        setPaused(!s.conditions.notPaused);
      } catch {}
    };
    poll();
    const id = setInterval(poll, 15000);
    return () => clearInterval(id);
  }, []);

  const togglePause = async () => {
    const newPaused = !paused;
    setPaused(newPaused);
    await api.setSchedulerPaused(newPaused);
  };

  return (
    <div className="scheduler-widget">
      <div className="sched-header">
        <span className="sched-label">Harmonogram</span>
        <div className={`sched-dot ${status?.isReady ? "ready" : "waiting"}`} />
      </div>
      {status?.blockedReason && (
        <div className="sched-reason">{status.blockedReason}</div>
      )}
      <button
        className={`sched-toggle ${paused ? "paused" : ""}`}
        onClick={togglePause}
      >
        {paused ? "▶ Wznów" : "⏸ Wstrzymaj"}
      </button>
    </div>
  );
};

// ─── Settings Page ────────────────────────────────────────────────────────────

const SettingsPage: React.FC = () => {
  return (
    <div className="settings-page">
      <h1 className="settings-title">Ustawienia</h1>
      <div className="settings-sections">
        <SettingsSection title="Harmonogram">
          <SettingRow
            label="Ćwiczenia dziennie"
            description="Maksymalna liczba wyskakujących okienek dziennie"
            control={<input type="number" defaultValue={50} min={5} max={200} className="setting-input" />}
          />
          <SettingRow
            label="Próg bezczynności"
            description="Sekundy bezczynności przed pokazaniem ćwiczenia"
            control={<input type="number" defaultValue={5} min={1} max={60} className="setting-input" />}
          />
          <SettingRow
            label="Minimalny odstęp między ćwiczeniami"
            description="Minuty przerwy między wyskakującymi okienkami"
            control={<input type="number" defaultValue={30} min={5} max={120} className="setting-input" />}
          />
        </SettingsSection>

        <SettingsSection title="Nauka">
          <SettingRow
            label="Uruchamiaj z Windowsem"
            description="Uruchom VocabTrainer automatycznie po zalogowaniu"
            control={<Toggle defaultOn />}
          />
          <SettingRow
            label="Pokaż słowo sesji"
            description="Wyświetl kartę wprowadzającą przy starcie komputera"
            control={<Toggle defaultOn />}
          />
          <SettingRow
            label="Efekty dźwiękowe"
            description="Odtwarzaj dźwięki przy poprawnych/błędnych odpowiedziach"
            control={<Toggle />}
          />
        </SettingsSection>

        <SettingsSection title="Godziny pracy">
          <SettingRow
            label="Tylko w godzinach pracy"
            description="Ogranicz ćwiczenia do określonych godzin"
            control={<Toggle defaultOn />}
          />
          <SettingRow
            label="Godziny pracy"
            description="Czas rozpoczęcia i zakończenia dostarczania ćwiczeń"
            control={
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input type="time" defaultValue="08:00" className="setting-input" />
                <span style={{ color: "var(--muted)" }}>do</span>
                <input type="time" defaultValue="22:00" className="setting-input" />
              </div>
            }
          />
        </SettingsSection>
      </div>
    </div>
  );
};

const SettingsSection: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <div className="settings-section">
    <h2 className="settings-section-title">{title}</h2>
    <div className="settings-rows">{children}</div>
  </div>
);

const SettingRow: React.FC<{ label: string; description: string; control: React.ReactNode }> = ({
  label, description, control,
}) => (
  <div className="setting-row">
    <div className="setting-info">
      <div className="setting-label">{label}</div>
      <div className="setting-desc">{description}</div>
    </div>
    <div className="setting-control">{control}</div>
  </div>
);

const Toggle: React.FC<{ defaultOn?: boolean }> = ({ defaultOn = false }) => {
  const [on, setOn] = useState(defaultOn);
  return (
    <button
      className={`toggle ${on ? "on" : ""}`}
      onClick={() => setOn(!on)}
      role="switch"
      aria-checked={on}
    >
      <div className="toggle-thumb" />
    </button>
  );
};
