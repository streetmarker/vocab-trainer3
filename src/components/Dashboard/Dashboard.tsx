// src/components/Dashboard/Dashboard.tsx
import React, { useEffect, useState } from "react";
import type { OverallStats, DailyStats, ActivityDay, Word } from "../../types";
import { DIFFICULTY_LABELS } from "../../types";
// MASTERY_COLORS
import { api } from "../../hooks/useTauri";
import "./Dashboard.css";

export const Dashboard: React.FC = () => {
  const [stats, setStats] = useState<OverallStats | null>(null);
  const [dailyStats, setDailyStats] = useState<DailyStats[]>([]);
  const [activity, setActivity] = useState<ActivityDay[]>([]);
  const [words, setWords] = useState<Word[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      try {
        const [s, d, a, w] = await Promise.all([
          api.getOverallStats(),
          api.getDailyStats(14),
          api.getActivityGrid(),
          api.getWords(),
        ]);
        setStats(s);
        setDailyStats(d);
        setActivity(a);
        setWords(w);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  if (loading) return <div className="dash-loading"><div className="spinner" /></div>;

  // const masteryBreakdown = words.reduce((acc, w) => {
  //   // We'd need progress for this; mock for now
  //   return acc;
  // }, {} as Record<string, number>);

  return (
    <div className="dashboard">
      {/* ── Header ─────────────────────────────────────────────────────── */}
      <div className="dash-header">
        <div>
          <h1 className="dash-title">Learning Dashboard</h1>
          <p className="dash-subtitle">
            {new Date().toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" })}
          </p>
        </div>
        <StreakBadge streak={stats?.currentStreak ?? 0} />
      </div>

      {/* ── Stats Grid ─────────────────────────────────────────────────── */}
      {stats && (
        <div className="stats-grid">
          <StatCard
            label="Total Words"
            value={stats.totalWords}
            icon="📖"
            color="#6c63ff"
          />
          <StatCard
            label="Mastered"
            value={stats.masteredWords}
            icon="🏆"
            color="#10b981"
            sub={`${stats.totalWords > 0 ? Math.round(stats.masteredWords / stats.totalWords * 100) : 0}% of library`}
          />
          <StatCard
            label="Exercises Done"
            value={stats.totalExercises}
            icon="⚡"
            color="#f59e0b"
          />
          <StatCard
            label="Accuracy"
            value={`${stats.accuracyPercent}%`}
            icon="🎯"
            color="#3b82f6"
            sub={`${stats.correctAnswers} / ${stats.totalExercises} correct`}
          />
        </div>
      )}

      {/* ── Activity Heatmap ─────────────────────────────────────────── */}
      <div className="dash-section">
        <h2 className="section-title">Activity (last 365 days)</h2>
        <ActivityHeatmap data={activity} />
      </div>

      {/* ── Last 14 days bar chart ────────────────────────────────────── */}
      <div className="dash-section">
        <h2 className="section-title">Daily Exercises</h2>
        <DailyChart data={dailyStats} />
      </div>

      {/* ── Word Library Breakdown ────────────────────────────────────── */}
      <div className="dash-section">
        <h2 className="section-title">Your Vocabulary ({words.length} words)</h2>
        <div className="word-grid">
          {words.slice(0, 12).map((w) => (
            <WordCard key={w.id} word={w} />
          ))}
        </div>
      </div>
    </div>
  );
};

// ─── Sub-components ───────────────────────────────────────────────────────────

const StatCard: React.FC<{
  label: string;
  value: string | number;
  icon: string;
  color: string;
  sub?: string;
}> = ({ label, value, icon, color, sub }) => (
  <div className="stat-card" style={{ "--accent": color } as React.CSSProperties}>
    <div className="stat-icon">{icon}</div>
    <div className="stat-value">{value}</div>
    <div className="stat-label">{label}</div>
    {sub && <div className="stat-sub">{sub}</div>}
  </div>
);

const StreakBadge: React.FC<{ streak: number }> = ({ streak }) => (
  <div className="streak-badge">
    <span className="streak-fire">🔥</span>
    <span className="streak-count">{streak}</span>
    <span className="streak-label">day streak</span>
  </div>
);

const ActivityHeatmap: React.FC<{ data: ActivityDay[] }> = ({ data }) => {
  const today = new Date();
  const weeks: (ActivityDay | null)[][] = [];

  // Build a 52-week grid
  for (let w = 51; w >= 0; w--) {
    const week: (ActivityDay | null)[] = [];
    for (let d = 6; d >= 0; d--) {
      const date = new Date(today);
      date.setDate(date.getDate() - (w * 7 + d));
      const dateStr = date.toISOString().split("T")[0];
      const day = data.find((a) => a.date === dateStr) ?? null;
      week.push(day ? { ...day, date: dateStr } : null);
    }
    weeks.push(week);
  }

  const maxCount = Math.max(...data.map((d) => d.count), 1);

  const getCellColor = (day: ActivityDay | null) => {
    if (!day || day.count === 0) return "var(--cell-empty)";
    const intensity = day.count / maxCount;
    if (intensity < 0.25) return "#312e81";
    if (intensity < 0.5)  return "#4c1d95";
    if (intensity < 0.75) return "#6c63ff";
    return "#a78bfa";
  };

  return (
    <div className="heatmap">
      {weeks.map((week, wi) => (
        <div key={wi} className="heatmap-week">
          {week.map((day, di) => (
            <div
              key={di}
              className="heatmap-cell"
              style={{ background: getCellColor(day) }}
              title={day ? `${day.date}: ${day.count} exercises` : "No activity"}
            />
          ))}
        </div>
      ))}
    </div>
  );
};

const DailyChart: React.FC<{ data: DailyStats[] }> = ({ data }) => {
  if (data.length === 0) {
    return <div className="empty-chart">No data yet — complete some exercises!</div>;
  }
  const max = Math.max(...data.map((d) => d.exercisesCompleted), 1);

  return (
    <div className="daily-chart">
      {data.map((d, i) => (
        <div key={i} className="bar-group">
          <div className="bar-wrap">
            <div
              className="bar"
              style={{ height: `${(d.exercisesCompleted / max) * 100}%` }}
              title={`${d.exercisesCompleted} exercises, ${d.correctAnswers} correct`}
            >
              <div
                className="bar-correct"
                style={{ height: `${d.exercisesCompleted > 0 ? (d.correctAnswers / d.exercisesCompleted) * 100 : 0}%` }}
              />
            </div>
          </div>
          <div className="bar-label">
            {new Date(d.date).toLocaleDateString("en-US", { month: "numeric", day: "numeric" })}
          </div>
        </div>
      ))}
    </div>
  );
};

const WordCard: React.FC<{ word: Word }> = ({ word }) => (
  <div className="word-card">
    <div className="wc-term">{word.term}</div>
    <div className="wc-pos">{word.partOfSpeech}</div>
    <div className="wc-def">{word.definition.slice(0, 60)}{word.definition.length > 60 ? "…" : ""}</div>
    <div className="wc-footer">
      <span
        className="wc-difficulty"
        style={{ color: ["#22c55e", "#84cc16", "#eab308", "#f97316", "#ef4444"][word.difficulty - 1] }}
      >
        {DIFFICULTY_LABELS[word.difficulty]}
      </span>
    </div>
  </div>
);
