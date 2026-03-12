// src/components/Flashcard/Flashcard.tsx
//
// SM-2 flashcard with 4-grade answer buttons.
//
// State machine:
//   initial  → front (Polish word), click to flip
//   flipped  → back (English word), 4 grade buttons visible
//   answered → locked, awaiting feedback prop to unmount/reset
//
// Grade buttons appear ONLY after flip, OUTSIDE the 3D scene (no perspective distortion).
// The parent (TaskNotification) owns the save/feedback/next cycle.

import React, { useState } from "react";
import "./Flashcard.css";

export type FlashcardPhase  = "initial" | "flipped" | "answered";
export type SrsGrade        = "again" | "hard" | "good" | "easy";

interface GradeButton {
  grade:   SrsGrade;
  label:   string;
  hint:    string;   // shown as tooltip / sub-label
}

const GRADE_BUTTONS: GradeButton[] = [
  { grade: "again", label: "Jeszcze raz", hint: "nie pamiętam" },
  { grade: "hard",  label: "Trudne",      hint: "z wysiłkiem"  },
  { grade: "good",  label: "Dobrze",      hint: "pamiętam"     },
  { grade: "easy",  label: "Łatwe",       hint: "od razu"      },
];

interface Props {
  front:        string;
  back:         string;
  hint?:        string;
  backLabel?:   string;
  onFlip?:      () => void;
  onAnswer?:    (grade: SrsGrade) => void;
  /** Locks card during async save / feedback display */
  disabled?:    boolean;
  /** Optional per-grade interval preview, e.g. { again: "10 min", hard: "1 dzień", … } */
  intervalHints?: Partial<Record<SrsGrade, string>>;
}

export function Flashcard({
  front, back, hint, backLabel,
  onFlip, onAnswer,
  disabled = false,
  intervalHints,
}: Props) {
  const [phase, setPhase] = useState<FlashcardPhase>("initial");

  const handleFlip = () => {
    if (disabled || phase === "answered") return;
    if (phase === "initial") {
      setPhase("flipped");
      onFlip?.();
    } else {
      // allow flip-back to re-read Polish
      setPhase("initial");
    }
  };

  const handleGrade = (grade: SrsGrade) => {
    if (disabled || phase !== "flipped") return;
    setPhase("answered");
    onAnswer?.(grade);
  };

  const isFlipped = phase === "flipped" || phase === "answered";

  return (
    <div className="fc-wrapper">
      {/* ── 3-D flip scene ─────────────────────────────────────────────── */}
      <div
        className={`fc-scene ${phase === "answered" ? "fc-scene--answered" : ""}`}
        onClick={handleFlip}
        role="button"
        tabIndex={0}
        aria-label={
          isFlipped
            ? `Angielski: ${back}. ${phase === "flipped" ? "Kliknij aby wrócić." : ""}`
            : `Polski: ${front}. Kliknij aby zobaczyć po angielsku.`
        }
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleFlip(); }
        }}
      >
        <div className={`fc-card ${isFlipped ? "fc-card--flipped" : ""}`}>
          {/* Front */}
          <div className="fc-face fc-face--front" aria-hidden={isFlipped}>
            <span className="fc-lang-badge">🇵🇱</span>
            <span className="fc-word">{front}</span>
            {hint && <span className="fc-hint">{hint}</span>}
            <span className="fc-flip-cue" aria-hidden>↻</span>
          </div>
          {/* Back */}
          <div className="fc-face fc-face--back" aria-hidden={!isFlipped}>
            <span className="fc-lang-badge">🇬🇧</span>
            {backLabel && <span className="fc-back-label">{backLabel}</span>}
            <span className="fc-word fc-word--back">{back}</span>
          </div>
        </div>
      </div>

      {/* ── Grade buttons — fade in after flip ─────────────────────────── */}
      <div
        className={`fc-grades ${phase === "flipped" ? "fc-grades--visible" : ""}`}
        role="group"
        aria-label="Oceń odpowiedź"
      >
        {GRADE_BUTTONS.map(({ grade, label, hint: btnHint }) => (
          <button
            key={grade}
            className={`fc-grade-btn fc-grade-btn--${grade}`}
            onClick={() => handleGrade(grade)}
            disabled={disabled}
            tabIndex={phase === "flipped" ? 0 : -1}
            title={btnHint}
          >
            <span className="fc-grade-label">{label}</span>
            {intervalHints?.[grade] && (
              <span className="fc-grade-interval">{intervalHints[grade]}</span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
