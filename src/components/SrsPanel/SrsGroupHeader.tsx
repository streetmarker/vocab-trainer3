// src/components/SrsPanel/SrsGroupHeader.tsx
// Sticky header for each word group in the grouped list view.
// Shows: icon + label + word count + optional "next review" time.

import React from "react";
import type { WordWithProgress } from "../../hooks/useTauri";
import { formatInterval } from "./srs-config";

interface Props {
  icon:     string;
  label:    string;
  color:    string;
  words:    WordWithProgress[];
  collapsed:   boolean;
  onToggle: () => void;
}

export function SrsGroupHeader({ icon, label, color, words, collapsed, onToggle }: Props) {
  // Earliest next_review_at in this group
  const nextDue = words
    .map(w => w.nextReviewAt ? new Date(w.nextReviewAt).getTime() : Infinity)
    .reduce((a, b) => Math.min(a, b), Infinity);

  const nextLabel = nextDue === Infinity
    ? null
    : (() => {
        const diffDays = (nextDue - Date.now()) / 86_400_000;
        return diffDays <= 0 ? "teraz" : formatInterval(diffDays);
      })();

  // Average ease factor
  const reviewed = words.filter(w => w.totalReviews > 0);
  const avgEase = reviewed.length > 0
    ? (reviewed.reduce((s, w) => s + w.easeFactor, 0) / reviewed.length).toFixed(1)
    : null;

  return (
    <button
      className="srs-group-header"
      style={{ "--group-color": color } as React.CSSProperties}
      onClick={onToggle}
      aria-expanded={!collapsed}
    >
      <span className="srs-group-header__icon">{icon}</span>
      <span className="srs-group-header__label">{label}</span>

      <div className="srs-group-header__meta">
        {nextLabel && (
          <span className="srs-group-header__next">następna: {nextLabel}</span>
        )}
        {avgEase && (
          <span className="srs-group-header__ease" title="Średni współczynnik łatwości (EF)">
            EF {avgEase}
          </span>
        )}
      </div>

      <span className="srs-group-header__count"
        style={{ background: `${color}20`, color, borderColor: `${color}35` }}
      >
        {words.length}
      </span>
      <span className="srs-group-header__chevron">{collapsed ? "▸" : "▾"}</span>
    </button>
  );
}
