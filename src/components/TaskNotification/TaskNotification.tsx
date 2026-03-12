// src/components/TaskNotification/TaskNotification.tsx

import React, { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Flashcard, type SrsGrade } from "../Flashcard/Flashcard";
import { api } from "../../hooks/useTauri";

const appWindow    = getCurrentWebviewWindow();
const AUTO_CLOSE   = 20_000;
const FEEDBACK_MS  = 1_200;
const SLIDE_OUT_MS = 320;

type SlidePhase = "in" | "idle" | "out";
type InnerPhase = "idle" | "flipped" | "saving" | "feedback";

type WordState = {
  wordId: number; termPl: string; termEn: string; partOfSpeech?: string;
};
type FeedbackState = {
  grade: SrsGrade; mastery: string; intervalLabel: string; streak: number;
};

const MASTERY_COLORS: Record<string, string> = {
  new: "#94a3b8", learning: "#a78bfa", reviewing: "#60a5fa", mastered: "#4ade80",
};
const MASTERY_LABELS: Record<string, string> = {
  new: "Nowe", learning: "W nauce", reviewing: "Utrwalanie", mastered: "Opanowane",
};
const GRADE_ICONS: Record<SrsGrade, string> = {
  again: "↩", hard: "〜", good: "✓", easy: "⚡",
};

type Props = {
  termPl: string; termEn: string; partOfSpeech?: string;
  wordId: number; onDismiss: () => void;
};

export function TaskNotification({ termPl, termEn, partOfSpeech, wordId, onDismiss }: Props) {
  const [slidePhase, setSlidePhase] = useState<SlidePhase>("in");
  const [innerPhase, setInnerPhase] = useState<InnerPhase>("idle");
  const [word, setWord]             = useState<WordState>({ wordId, termPl, termEn, partOfSpeech });
  const [feedback, setFeedback]     = useState<FeedbackState | null>(null);
  const [progress, setProgress]     = useState(100);
  const [cardKey, setCardKey]       = useState(0);

  const rafRef     = useRef<number>(0);
  const startRef   = useRef<number>(0);
  const elapsedRef = useRef<number>(0);
  const pausedRef  = useRef(false);
  const closedRef  = useRef(false);

  const hardClose = useCallback(() => {
    if (closedRef.current) return;
    closedRef.current = true;
    cancelAnimationFrame(rafRef.current);
    setSlidePhase("out");
    setTimeout(async () => { onDismiss(); await appWindow.hide(); }, SLIDE_OUT_MS);
  }, [onDismiss]);

  const handleLater = useCallback(async () => {
    try { await api.taskNotificationLater(word.wordId); } catch { /* ignore */ }
    hardClose();
  }, [word.wordId, hardClose]);

  const handleCardFlip = useCallback(() => {
    setInnerPhase("flipped");
    pausedRef.current   = true;
    elapsedRef.current += performance.now() - startRef.current;
  }, []);

  const handleGrade = useCallback(async (grade: SrsGrade) => {
    setInnerPhase("saving");
    cancelAnimationFrame(rafRef.current);

    try {
      const result = await api.srsAnswer(word.wordId, grade);

      setFeedback({
        grade,
        mastery:       result.newMastery,
        intervalLabel: result.nextReviewLabel,
        streak:        result.streak,
      });
      setInnerPhase("feedback");

      setTimeout(() => {
        if (closedRef.current) return;

        if (result.nextWordId && result.nextTermPl && result.nextTermEn) {
          setWord({
            wordId:       result.nextWordId,
            termPl:       result.nextTermPl,
            termEn:       result.nextTermEn,
            partOfSpeech: result.nextPartOfSpeech ?? undefined,
          });
          setFeedback(null);
          setInnerPhase("idle");
          setCardKey(k => k + 1);
          setProgress(100);
          elapsedRef.current = 0;
          pausedRef.current  = false;
          startRef.current   = performance.now();
          rafRef.current     = requestAnimationFrame(tick);
        } else {
          hardClose();
        }
      }, FEEDBACK_MS);

    } catch (err) {
      console.error("srs_answer failed:", err);
      setInnerPhase("flipped");
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [word.wordId, hardClose]);

  const tick = useCallback(() => {
    if (closedRef.current) return;
    if (!pausedRef.current) {
      const total = elapsedRef.current + (performance.now() - startRef.current);
      const pct   = Math.max(0, 100 - (total / AUTO_CLOSE) * 100);
      setProgress(pct);
      if (pct <= 0) { handleLater(); return; }
    }
    rafRef.current = requestAnimationFrame(tick);
  }, [handleLater]);

  useEffect(() => {
    const t = setTimeout(() => setSlidePhase("idle"), 16);
    startRef.current = performance.now();
    rafRef.current   = requestAnimationFrame(tick);
    return () => { clearTimeout(t); cancelAnimationFrame(rafRef.current); };
  }, [tick]);

  const onMouseEnter = () => {
    if (closedRef.current || innerPhase !== "idle") return;
    pausedRef.current   = true;
    elapsedRef.current += performance.now() - startRef.current;
  };
  const onMouseLeave = () => {
    if (closedRef.current || innerPhase !== "idle") return;
    pausedRef.current = false;
    startRef.current  = performance.now();
  };

  const isFlipped = innerPhase !== "idle";

  return (
    <div
      className={`tn-root tn-root--${slidePhase}`}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {/* Progress bar */}
      <div className="tn-progress">
        <div className="tn-progress__fill" style={{
          width: `${progress}%`,
          opacity: isFlipped ? 0 : 1,
          transition: isFlipped ? "opacity 0.3s" : undefined,
        }} />
      </div>

      {/* Header */}
      <div className="tn-header">
        <span className="tn-icon" aria-hidden>📚</span>
        <span className="tn-title">Nauka słówek</span>
        <button className="tn-close" aria-label="Zamknij" onClick={handleLater}>×</button>
      </div>

      {/* Flashcard */}
      <div className="tn-card-wrap">
        <Flashcard
          key={cardKey}
          front={word.termPl}
          back={word.termEn}
          backLabel={word.partOfSpeech}
          hint="kliknij aby zobaczyć po angielsku"
          onFlip={handleCardFlip}
          onAnswer={handleGrade}
          disabled={innerPhase === "saving" || innerPhase === "feedback"}
        />
      </div>

      {/* Feedback badge */}
      {feedback && (
        <div
          className="tn-feedback"
          style={{ "--mastery-color": MASTERY_COLORS[feedback.mastery] ?? "#94a3b8" } as React.CSSProperties}
        >
          <span className="tn-feedback__icon">{GRADE_ICONS[feedback.grade]}</span>
          <span className="tn-feedback__mastery">{MASTERY_LABELS[feedback.mastery] ?? feedback.mastery}</span>
          <span className="tn-feedback__interval">{feedback.intervalLabel}</span>
          {feedback.streak > 1 && <span className="tn-feedback__streak">🔥 {feedback.streak}</span>}
        </div>
      )}

      {/* Pre-flip action — only Później remains */}
      <div className={`tn-actions ${isFlipped ? "tn-actions--hidden" : ""}`}>
        <button className="tn-btn tn-btn--later tn-btn--full" onClick={handleLater}>Później</button>
      </div>
    </div>
  );
}
