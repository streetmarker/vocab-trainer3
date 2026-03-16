// src/components/SrsPanel/SrsBadge.tsx
// Compact inline badge shown on each VocabRow.
// Shows mastery level + interval + streak (if > 1).

import type { WordWithProgress } from "../../hooks/useTauri";
import { MASTERY_CONFIG, REVIEW_STATUS_CONFIG, formatInterval, formatAgo } from "./srs-config";

interface Props {
  word: WordWithProgress;
}

export function SrsBadge({ word }: Props) {
  const mastery  = MASTERY_CONFIG[word.masteryLevel];
  const rstatus  = REVIEW_STATUS_CONFIG[word.reviewStatus];

  const intervalLabel = word.totalReviews === 0
    ? "nowe"
    : word.reviewStatus === "overdue" || word.reviewStatus === "today"
      ? rstatus.label.toLowerCase()
      : formatInterval(word.intervalDays);

  const lastLabel = word.lastReviewAt ? formatAgo(word.lastReviewAt) : null;

  return (
    <div className="srs-badge" title={lastLabel ? `Ostatnia powtórka: ${lastLabel}` : "Nie powtarzane"}>
      {/* Mastery pill */}
      <span
        className="srs-badge__mastery"
        style={{ background: mastery.bg, color: mastery.color, borderColor: `${mastery.color}35` }}
      >
        <span className="srs-badge__mastery-icon">{mastery.icon}</span>
        {mastery.label}
      </span>

      {/* Interval */}
      <span
        className={`srs-badge__interval ${rstatus.urgent ? "srs-badge__interval--urgent" : ""}`}
        style={{ color: rstatus.urgent ? rstatus.color : undefined }}
      >
        {intervalLabel}
      </span>

      {/* Streak fire (only if > 1) */}
      {word.streak > 1 && (
        <span className="srs-badge__streak" title={`Passa: ${word.streak}`}>
          🔥{word.streak}
        </span>
      )}

      {/* Repetitions dot-bar (max 8 shown) */}
      <span className="srs-badge__reps" title={`Powtórzeń: ${word.repetitions}`}>
        {Array.from({ length: Math.min(word.repetitions, 8) }).map((_, i) => (
          <span key={i} className="srs-badge__rep-dot" />
        ))}
        {word.repetitions > 8 && <span className="srs-badge__rep-more">+</span>}
      </span>
    </div>
  );
}
