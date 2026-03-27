// src/components/Dashboard/Dashboard.tsx
import React, { useEffect, useState } from "react";
import type { OverallStats, ActivityDay, Word } from "../../types";
import { DIFFICULTY_LABELS, DIFFICULTY_COLORS, PART_OF_SPEECH_LABELS } from "../../types";
import { api } from "../../hooks/useTauri";
import "./Dashboard.css";

export const Dashboard: React.FC = () => {
  const [stats, setStats]           = useState<OverallStats | null>(null);
  const [activity, setActivity]     = useState<ActivityDay[]>([]);
  const [words, setWords]           = useState<Word[]>([]);
  const [currentWord, setCurrentWord] = useState<Word | null>(null);
  const [loading, setLoading]       = useState(true);

  useEffect(() => {
    Promise.all([
      api.getOverallStats(),
      api.getActivityGrid(),
      api.getWords(),
      api.getCurrentWord(),
    ]).then(([s, a, w, cw]) => {
      setStats(s); setActivity(a); setWords(w); setCurrentWord(cw); setLoading(false);
    }).catch(console.error);
  }, []);

  if (loading) return (
    <div className="dash-loading">
      <div className="spinner" />
    </div>
  );

  return (
    <div className="dashboard">

      {/* Header */}
      <div className="dash-header">
        <div>
          <h1 className="dash-title">Panel główny</h1>
          <p className="dash-subtitle">Twoje postępy w nauce angielskiego</p>
        </div>
        {stats && (
          <div className="streak-badge">
            <span className="streak-fire">🔥</span>
            <span className="streak-count">{stats.currentStreak}</span>
            <span className="streak-label">dni z rzędu</span>
          </div>
        )}
      </div>

      {/* Current word widget */}
      {currentWord && (
        <div className="current-word-banner">
          <div className="cw-label">Aktualnie ćwiczone</div>
          <div className="cw-term">{currentWord.term}</div>
          {currentWord.phonetic && <div className="cw-phonetic">{currentWord.phonetic}</div>}
          <div className="cw-def">{currentWord.definition}</div>
          {currentWord.definitionPl && (
            <div className="cw-def-pl"><span className="pl-flag">🇵🇱</span>{currentWord.definitionPl}</div>
          )}
        </div>
      )}

      {/* Stat cards */}
      {stats && (
        <div className="stats-grid">
          <StatCard label="Słowa w bazie"  value={stats.totalWords}      icon="📚" color="#6c63ff" sub="aktywnych słów" />
          <StatCard label="Opanowane"      value={stats.masteredWords}   icon="🏆" color="#10b981" sub="w pełni przyswojone" />
          <StatCard label="Ćwiczenia"      value={stats.totalExercises}  icon="✏️" color="#3b82f6" sub="łącznie wykonanych" />
          <StatCard label="Skuteczność"    value={`${stats.accuracyPercent}%`} icon="🎯" color="#f59e0b" sub="poprawnych odpowiedzi" />
        </div>
      )}

      {/* Activity heatmap */}
      <div className="dash-section">
        <h2 className="section-title">Aktywność (ostatni rok)</h2>
        <ActivityHeatmap data={activity} />
      </div>

      {/* Word list */}
      <div className="dash-section">
        <h2 className="section-title">Twoje słowa ({words.length})</h2>
        {words.length === 0 ? (
          <div className="empty-chart">
            Brak słów. Przejdź do <strong>Słownictwo</strong> aby dodać pierwsze słowa lub załadować przykładowe.
          </div>
        ) : (
          <div className="word-grid">
            {words.map((w) => (
              <WordCard
                key={w.id}
                word={w}
                onDelete={async (id) => {
                  if (confirm(`Czy na pewno usunąć słowo "${w.term}"?`)) {
                    try {
                      await api.deleteWord(id);
                      setWords(prev => prev.filter(item => item.id !== id));
                    } catch (e) {
                      console.error("Failed to delete word:", e);
                      alert("Błąd podczas usuwania słowa.");
                    }
                  }
                }}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

// ─── Sub-components ───────────────────────────────────────────────────────────

const StatCard: React.FC<{ label: string; value: string | number; icon: string; color: string; sub: string }> =
  ({ label, value, icon, color, sub }) => (
    <div className="stat-card" style={{ "--accent": color } as React.CSSProperties}>
      <span className="stat-icon">{icon}</span>
      <span className="stat-value" style={{ color }}>{value}</span>
      <span className="stat-label">{label}</span>
      <span className="stat-sub">{sub}</span>
    </div>
  );

const ActivityHeatmap: React.FC<{ data: ActivityDay[] }> = ({ data }) => {
  const byDate = new Map(data.map((d) => [d.date, d]));
  const today = new Date();
  const start = new Date(today);
  start.setDate(today.getDate() - 364);

  const weeks: (ActivityDay | null)[][] = [];
  let week: (ActivityDay | null)[] = Array(start.getDay()).fill(null);

  for (let d = new Date(start); d <= today; d.setDate(d.getDate() + 1)) {
    const key = d.toISOString().slice(0, 10);
    week.push(byDate.get(key) ?? { date: key, count: 0, correct: 0 });
    if (d.getDay() === 6) { weeks.push(week); week = []; }
  }
  if (week.length) weeks.push(week);

  const maxCount = Math.max(...data.map((d) => d.count), 1);

  return (
    <div className="heatmap">
      {weeks.map((week, wi) => (
        <div key={wi} className="heatmap-week">
          {week.map((day, di) =>
            day ? (
              <div
                key={di}
                className="heatmap-cell"
                style={{
                  background: "#6c63ff",
                  opacity: day.count === 0 ? 0.07 : 0.2 + 0.8 * (day.count / maxCount),
                }}
                title={`${day.date}: ${day.count} ćwiczeń`}
              />
            ) : (
              <div key={di} className="heatmap-cell" style={{ background: "transparent" }} />
            )
          )}
        </div>
      ))}
    </div>
  );
};

const WordCard: React.FC<{ word: Word; onDelete: (id: number) => void }> = ({ word, onDelete }) => (
  <div className="word-card">
    <button
      className="wc-delete-btn"
      onClick={(e) => {
        e.stopPropagation();
        onDelete(word.id);
      }}
      title="Usuń słowo"
      aria-label="Usuń słowo"
    >
      ✕
    </button>
    <span className="wc-term">{word.term}</span>
    <span className="wc-pos">{PART_OF_SPEECH_LABELS[word.partOfSpeech] ?? word.partOfSpeech}</span>
    <span className="wc-def">{word.definition}</span>
    {word.definitionPl && (
      <div className="word-card-pl">
        <span className="pl-flag">🇵🇱</span>
        {word.definitionPl}
      </div>
    )}
    <div className="wc-footer">
      <span className="wc-difficulty" style={{ color: DIFFICULTY_COLORS[word.difficulty] }}>
        {DIFFICULTY_LABELS[word.difficulty]}
      </span>
    </div>
  </div>
);
