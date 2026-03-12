// src/components/SrsPanel/srs-config.ts
// Central config for SRS display — edit once, updates everywhere.

import type { SrsMastery, SrsReviewStatus } from "../../hooks/useTauri";

export const MASTERY_CONFIG: Record<SrsMastery, {
  label:   string;
  color:   string;   // CSS custom-property value
  bg:      string;
  icon:    string;
  order:   number;   // sort priority in grouped view (lower = shown first)
}> = {
  new:       { label: "Nowe",       color: "#94a3b8", bg: "rgba(148,163,184,0.10)", icon: "✦", order: 1 },
  learning:  { label: "W nauce",    color: "#a78bfa", bg: "rgba(167,139,250,0.12)", icon: "◎", order: 2 },
  reviewing: { label: "Powtórka",   color: "#60a5fa", bg: "rgba(96,165,250,0.12)",  icon: "↻", order: 3 },
  mastered:  { label: "Opanowane",  color: "#4ade80", bg: "rgba(74,222,128,0.10)",  icon: "✓", order: 4 },
};

export const REVIEW_STATUS_CONFIG: Record<SrsReviewStatus, {
  label:  string;
  color:  string;
  urgent: boolean;
}> = {
  overdue: { label: "Zaległe",  color: "#f87171", urgent: true  },
  today:   { label: "Dziś",     color: "#fbbf24", urgent: true  },
  future:  { label: "Zaplanowane", color: "#60a5fa", urgent: false },
  never:   { label: "Nowe",     color: "#94a3b8", urgent: false },
};

/** Groups used in the grouped-list view, in display order. */
export const SRS_GROUPS = [
  {
    id:      "due" as const,
    label:   "Do powtórki",
    icon:    "🔔",
    color:   "#f87171",
    filter:  (w: { reviewStatus: SrsReviewStatus }) =>
      w.reviewStatus === "overdue" || w.reviewStatus === "today",
  },
  {
    id:      "learning" as const,
    label:   "W trakcie nauki",
    icon:    "◎",
    color:   "#a78bfa",
    filter:  (w: { masteryLevel: SrsMastery; reviewStatus: SrsReviewStatus }) =>
      w.masteryLevel === "learning" && w.reviewStatus === "future",
  },
  {
    id:      "reviewing" as const,
    label:   "Utrwalanie",
    icon:    "↻",
    color:   "#60a5fa",
    filter:  (w: { masteryLevel: SrsMastery; reviewStatus: SrsReviewStatus }) =>
      w.masteryLevel === "reviewing" && w.reviewStatus === "future",
  },
  {
    id:      "mastered" as const,
    label:   "Opanowane",
    icon:    "✓",
    color:   "#4ade80",
    filter:  (w: { masteryLevel: SrsMastery }) => w.masteryLevel === "mastered",
  },
  {
    id:      "new" as const,
    label:   "Nowe słowa",
    icon:    "✦",
    color:   "#94a3b8",
    filter:  (w: { masteryLevel: SrsMastery }) => w.masteryLevel === "new",
  },
] as const;

/** Format fractional-day interval into Polish string. */
export function formatInterval(days: number): string {
  const m = Math.round(days * 24 * 60);
  if (m < 1)    return "teraz";
  if (m < 60)   return `${m} min`;
  if (m < 1440) return `${Math.round(m / 60)} h`;
  const d = Math.round(m / 1440);
  return d === 1 ? "jutro" : `${d} dni`;
}

/** How long ago (for last_review_at). */
export function formatAgo(isoDate: string): string {
  const diff = Date.now() - new Date(isoDate).getTime();
  const d = Math.floor(diff / 86_400_000);
  if (d === 0) return "dziś";
  if (d === 1) return "wczoraj";
  return `${d} dni temu`;
}
