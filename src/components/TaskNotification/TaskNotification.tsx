// src/components/TaskNotification/TaskNotification.tsx

import React, { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Flashcard, type SrsGrade } from "../Flashcard/Flashcard";
import { api } from "../../hooks/useTauri";
import { formatReviewDate } from "../../utils/date";
import TtsPlayer from "../TtsPlayer";

const appWindow    = getCurrentWebviewWindow();
const AUTO_CLOSE   = 20_000;
const FEEDBACK_MS  = 1_200;
const SLIDE_OUT_MS = 320;

type SlidePhase = "in" | "idle" | "out";
type InnerPhase = "idle" | "flipped" | "saving" | "feedback";

type WordState = {
  wordId: number; termPl: string; termEn: string; partOfSpeech?: string;
  phonetic?: string | null;
  sentencePl?: string | null; sentenceEn?: string | null;
};
type FeedbackState = {
  grade: SrsGrade; mastery: string; nextReviewAt: string; streak: number;
};

const MASTERY_COLORS: Record<string, string> = {
  new: "#94a3b8", learning: "#a78bfa", reviewing: "#60a5fa", mastered: "#4ade80",
};
const MASTERY_LABELS: Record<string, string> = {
  new: "Nowe", learning: "W nauce", reviewing: "Powtórka", mastered: "Opanowane",
  complete: "Sesja ukończona! 🎉"
};

const GRADE_ICONS: Record<string, string> = {
  again: "🔄", hard: "🧠", good: "✅", easy: "✨",
};
/**
/**
 * Parses **text** markers and returns a React node with those segments
 * wrapped in <strong>. Renders all occurrences. Falls back to plain text
 * if no markers are found.
 */
function parseBold(sentence: string): React.ReactNode {
  if (sentence.includes("--")) {
    sentence = sentence.replace(/--/g, "**");
  }
  if (!sentence.includes("**")) return sentence;
  // else if includes "--" replace that with "**"

  const parts = sentence.split(/(\*\*[^*]+\*\*)/g);
  return (
    <>
      {parts.map((part, i) =>
        part.startsWith("**") && part.endsWith("**")
          ? <strong key={i}>{part.slice(2, -2)}</strong>
          : part
      )}
    </>
  );
}
// function to trim "--" and "**" from string
function trimMarkers(input: string): string {
  return input.replace(/(\*\*|--)/g, "");
}


type Props = {
  termPl: string; termEn: string; partOfSpeech?: string;
  phonetic?: string | null;
  sentencePl?: string | null; sentenceEn?: string | null;
  wordId: number; onDismiss: () => void;
};

export function TaskNotification({ termPl, termEn, partOfSpeech, phonetic, sentencePl, sentenceEn, wordId, onDismiss }: Props) {
  const [slidePhase, setSlidePhase] = useState<SlidePhase>("in");
  const [innerPhase, setInnerPhase] = useState<InnerPhase>("idle");
  const [word, setWord]             = useState<WordState>({ wordId, termPl, termEn, partOfSpeech, phonetic, sentencePl, sentenceEn });
  const [feedback, setFeedback]     = useState<FeedbackState | null>(null);
  const [progress, setProgress]     = useState(100);
  const [cardKey, setCardKey]       = useState(0);

  const rafRef     = useRef<number>(0);
  const startRef   = useRef<number>(0);
  const elapsedRef = useRef<number>(0);
  const pausedRef  = useRef(false);
  const closedRef  = useRef(false);
  const isHoveringRef = useRef(false);

  const hardClose = useCallback((skipGapReset = false) => {
    if (closedRef.current) return;
    closedRef.current = true;
    cancelAnimationFrame(rafRef.current);
    setSlidePhase("out");
    setTimeout(async () => {
      // Reset the scheduler gap timer unless the caller already did it
      // (handleLater calls task_notification_later explicitly before hardClose).
      if (!skipGapReset) {
        try { await api.taskNotificationLater(word.wordId); } catch { /* ignore */ }
      }
      onDismiss();
      await appWindow.hide();
    }, SLIDE_OUT_MS);
  }, [onDismiss, word.wordId]);

  const handleLater = useCallback(async () => {
    try { await api.taskNotificationLater(word.wordId); } catch { /* ignore */ }
    hardClose(true); // skipGapReset=true — already called taskNotificationLater above
  }, [word.wordId, hardClose]);

  const handleCardFlip = useCallback(() => {
    setInnerPhase("flipped");
    // We do NOT pause here anymore.
    // The user requested: "mouse on popup -> pause, mouse out -> run, REGARDLESS of other functionality".
    // So if you flip but are NOT hovering, the timer should run.
  }, []);

  const handleGrade = useCallback(async (grade: SrsGrade) => {
    setInnerPhase("saving");
    cancelAnimationFrame(rafRef.current);

    try {
      const result = await api.srsAnswer(word.wordId, grade);

      setFeedback({
        grade,
        mastery:       result.newMastery,
        nextReviewAt:  result.nextReviewAt,
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
            phonetic:     result.nextPhonetic ?? null,
            sentencePl:   result.nextSentencePl ?? null,
            sentenceEn:   result.nextSentenceEn ?? null,
          });
          setFeedback(null);
          setInnerPhase("idle");
          setCardKey(k => k + 1);
          setProgress(100);
          elapsedRef.current = 0;
          
          // STRICT TIMER LOGIC:
          // Resume ONLY if not hovering.
          // If hovering, we stay paused (pausedRef=true).
          pausedRef.current = isHoveringRef.current;
          
          if (!pausedRef.current) {
            startRef.current = performance.now();
          }
          
          rafRef.current = requestAnimationFrame(tick);
        } else {
          // Koniec kolejki - ładny komunikat przed zamknięciem
          setInnerPhase("feedback");
          setFeedback({
            grade: "good",
            mastery: "complete", // Klucz do etykiety 'Sesja ukończona!'
            nextReviewAt: new Date().toISOString(),
            streak: 0
          });
          setTimeout(hardClose, 2000);
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
    if (closedRef.current) return;
    isHoveringRef.current = true;
    
    // Always pause on hover, regardless of phase
    if (!pausedRef.current) {
      pausedRef.current = true;
      elapsedRef.current += performance.now() - startRef.current;
    }
  };

  const onMouseLeave = () => {
    if (closedRef.current) return;
    isHoveringRef.current = false;

    // Always resume on leave, regardless of phase
    if (pausedRef.current) {
      pausedRef.current = false;
      startRef.current = performance.now();
    }
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
          front={word.termEn}
          back={word.termEn}
          frontNode={
            <div className="fc-front-rich">
              <strong className="fc-front-definition">{word.termPl}</strong>
              <div className="fc-front-sentence">{parseBold(word.sentencePl ?? "")}</div>
            </div>
          }
          backNode={
            <div className="fc-back-rich">
              <div className="fc-back-header">
                <span className="fc-back-term">{word.termEn}</span>
                {word.phonetic && <span className="fc-back-phonetic">{word.phonetic}</span>}
                {word.partOfSpeech && <span className="fc-back-pos">{word.partOfSpeech}</span>}
              </div>
              <div className="fc-back-sentence">{parseBold(word.sentenceEn ?? "")}</div>
            </div>
          }
          term={word.termEn}
          exampleEn={trimMarkers(word.sentenceEn ?? "")}
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
          <span className="tn-feedback__interval">{formatReviewDate(feedback.nextReviewAt)}</span>
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
