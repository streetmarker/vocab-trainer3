// src/components/SrsPanel/SrsToday.tsx
// "Stan nauki dziś" — compact stats bar at the top of VocabManager.

import React from "react";
import type { SrsTodayStats } from "../../hooks/useTauri";

interface Props {
  stats: SrsTodayStats;
  loading?: boolean;
}

interface StatCell {
  label:    string;
  value:    number;
  color:    string;
  icon:     string;
  urgent?:  boolean;
}

export function SrsToday({ stats, loading = false }: Props) {
  if (loading) {
    return <div className="srs-today srs-today--loading"><span className="srs-skeleton" /></div>;
  }

  const cells: StatCell[] = [
    { label: "Do powtórki",  value: stats.dueToday, color: "#f87171", icon: "🔔", urgent: stats.dueToday > 0 },
    { label: "Nowe",         value: stats.newWords,  color: "#94a3b8", icon: "✦" },
    { label: "W nauce",      value: stats.learning,  color: "#a78bfa", icon: "◎" },
    { label: "Utrwalanie",   value: stats.reviewing, color: "#60a5fa", icon: "↻" },
    { label: "Opanowane",    value: stats.mastered,  color: "#4ade80", icon: "✓" },
  ];

  // Progress toward mastery — what % of total is mastered or reviewing
  const progressPct = stats.total > 0
    ? Math.round(((stats.mastered + stats.reviewing) / stats.total) * 100)
    : 0;

  return (
    <div className="srs-today">
      <div className="srs-today__header">
        <span className="srs-today__title">Stan nauki dziś</span>
        <span className="srs-today__total">{stats.total} słów łącznie</span>
      </div>

      {/* Stat cells */}
      <div className="srs-today__cells">
        {cells.map(({ label, value, color, icon, urgent }) => (
          <div
            key={label}
            className={`srs-stat-cell ${urgent ? "srs-stat-cell--urgent" : ""}`}
            style={{ "--cell-color": color } as React.CSSProperties}
          >
            <span className="srs-stat-cell__icon">{icon}</span>
            <span className="srs-stat-cell__value">{value}</span>
            <span className="srs-stat-cell__label">{label}</span>
          </div>
        ))}
      </div>

      {/* Mastery progress bar */}
      {stats.total > 0 && (
        <div className="srs-today__progress-wrap">
          <div className="srs-today__progress-track">
            {/* Layered bar: mastered (green) + reviewing (blue) + learning (purple) */}
            <div
              className="srs-today__progress-fill srs-today__progress-fill--mastered"
              style={{ width: `${(stats.mastered / stats.total) * 100}%` }}
            />
            <div
              className="srs-today__progress-fill srs-today__progress-fill--reviewing"
              style={{ width: `${((stats.mastered + stats.reviewing) / stats.total) * 100}%` }}
            />
            <div
              className="srs-today__progress-fill srs-today__progress-fill--learning"
              style={{ width: `${((stats.mastered + stats.reviewing + stats.learning) / stats.total) * 100}%` }}
            />
          </div>
          <span className="srs-today__progress-label">{progressPct}% opanowane lub w powtórce</span>
        </div>
      )}
    </div>
  );
}
