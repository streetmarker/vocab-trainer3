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
        console.error("Failed to load exercise:", err);
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
        console.error("Session start failed:", err);
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
    { id: "dashboard", icon: "📊", label: "Dashboard" },
    { id: "vocab", icon: "📚", label: "Vocabulary" },
    { id: "settings", icon: "⚙️", label: "Settings" },
  ];

  return (
    <div className="app-root">
      {/* ── Sidebar ───────────────────────────────────────────────────── */}
      <nav className="sidebar">
        <div className="sidebar-logo">
          <div className="logo-mark">V</div>
          <div>
            <div className="logo-name">VocabTrainer</div>
            <div className="logo-tagline">Micro Learning</div>
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
            ? `✓ Correct! +${showResult.streak > 1 ? ` ${showResult.streak} streak!` : ""}`
            : `Next review in ${(showResult.newIntervalDays * 24).toFixed(0)}h`}
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
        <span className="sched-label">Scheduler</span>
        <div className={`sched-dot ${status?.isReady ? "ready" : "waiting"}`} />
      </div>
      {status?.blockedReason && (
        <div className="sched-reason">{status.blockedReason}</div>
      )}
      <button
        className={`sched-toggle ${paused ? "paused" : ""}`}
        onClick={togglePause}
      >
        {paused ? "▶ Resume" : "⏸ Pause"}
      </button>
    </div>
  );
};

// ─── Settings Page ────────────────────────────────────────────────────────────

const SettingsPage: React.FC = () => {
  return (
    <div className="settings-page">
      <h1 className="settings-title">Settings</h1>
      <div className="settings-sections">
        <SettingsSection title="Scheduler">
          <SettingRow
            label="Exercises per day"
            description="Maximum number of popups per day"
            control={<input type="number" defaultValue={50} min={5} max={200} className="setting-input" />}
          />
          <SettingRow
            label="Idle threshold"
            description="Seconds of inactivity before showing an exercise"
            control={<input type="number" defaultValue={5} min={1} max={60} className="setting-input" />}
          />
          <SettingRow
            label="Minimum gap between exercises"
            description="Minutes between popups"
            control={<input type="number" defaultValue={30} min={5} max={120} className="setting-input" />}
          />
        </SettingsSection>

        <SettingsSection title="Learning">
          <SettingRow
            label="Autostart with Windows"
            description="Launch VocabTrainer when you log in"
            control={<Toggle defaultOn />}
          />
          <SettingRow
            label="Show word of the session"
            description="Display intro card when computer starts"
            control={<Toggle defaultOn />}
          />
          <SettingRow
            label="Sound effects"
            description="Play sounds on correct/incorrect answers"
            control={<Toggle />}
          />
        </SettingsSection>

        <SettingsSection title="Work Hours">
          <SettingRow
            label="Only show during work hours"
            description="Restrict exercises to specific hours"
            control={<Toggle defaultOn />}
          />
          <SettingRow
            label="Work hours"
            description="Start and end time for exercise delivery"
            control={
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input type="time" defaultValue="08:00" className="setting-input" />
                <span style={{ color: "var(--muted)" }}>to</span>
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
