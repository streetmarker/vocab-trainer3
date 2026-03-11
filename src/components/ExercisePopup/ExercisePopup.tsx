// src/components/ExercisePopup/ExercisePopup.tsx
import React, { useState, useCallback, useRef } from "react";
import type {
  Exercise, IntroductionExercise, MultipleChoiceExercise,
  FillInBlankExercise, TrueFalseExercise, DefinitionRecallExercise,
  ContextualGuessExercise, AnswerResult,
} from "../../types";
import { api } from "../../hooks/useTauri";
import "./ExercisePopup.css";

interface Props {
  exercise: Exercise;
  onComplete: (result: AnswerResult) => void;
  onDismiss: () => void;
}

// Maps Rust PascalCase tag → snake_case for the backend ExerciseType enum
function toExerciseTypeStr(type_: string): string {
  return type_.replace(/([A-Z])/g, (c, _, i) => (i === 0 ? c.toLowerCase() : `_${c.toLowerCase()}`));
}

export const ExercisePopup: React.FC<Props> = ({ exercise, onComplete, onDismiss }) => {
  const [answered, setAnswered] = useState(false);
  const [wasCorrect, setWasCorrect] = useState(false);
  const [result, setResult] = useState<AnswerResult | null>(null);
  const [isExiting, setIsExiting] = useState(false);
  const startTimeRef = useRef(Date.now());

  const handleAnswer = useCallback(async (correct: boolean, userAnswer?: string) => {
    if (answered) return;
    const responseTimeMs = Date.now() - startTimeRef.current;
    setAnswered(true);
    setWasCorrect(correct);

    try {
      const res = await api.submitAnswer({
        wordId: exercise.wordId,
        wasCorrect: correct,
        responseTimeMs,
        userAnswer,
        exerciseType: toExerciseTypeStr(exercise.type),
      });
      setResult(res);
      setTimeout(() => {
        setIsExiting(true);
        setTimeout(() => onComplete(res), 300);
      }, correct ? 1200 : 2200);
    } catch (err) {
      console.error("Failed to submit answer:", err);
    }
  }, [answered, exercise, onComplete]);

  const handleDismiss = () => {
    setIsExiting(true);
    setTimeout(onDismiss, 300);
  };

  // Introduction auto-dismisses after "Got it" click
  const isIntro = exercise.type === "Introduction";

  return (
    <div className={`popup-root ${isExiting ? "exiting" : "entering"}`}>
      <div className="popup-header">
        <div className="popup-logo">
          <span className="logo-icon">📚</span>
          <span className="logo-text">VocabTrainer</span>
        </div>
        <button className="popup-dismiss" onClick={handleDismiss} aria-label="Dismiss">✕</button>
      </div>

      <div className="popup-body">
        {exercise.type === "Introduction" && (
          <IntroductionView
            exercise={exercise as IntroductionExercise}
            onAcknowledge={() => handleAnswer(true, "acknowledged")}
          />
        )}
        {exercise.type === "MultipleChoice" && (
          <MultipleChoiceView
            exercise={exercise as MultipleChoiceExercise}
            answered={answered}
            onAnswer={handleAnswer}
          />
        )}
        {exercise.type === "TrueFalse" && (
          <TrueFalseView
            exercise={exercise as TrueFalseExercise}
            answered={answered}
            onAnswer={handleAnswer}
          />
        )}
        {exercise.type === "DefinitionRecall" && (
          <DefinitionRecallView
            exercise={exercise as DefinitionRecallExercise}
            answered={answered}
            onAnswer={handleAnswer}
          />
        )}
        {exercise.type === "ContextualGuess" && (
          <ContextualGuessView
            exercise={exercise as ContextualGuessExercise}
            answered={answered}
            onAnswer={handleAnswer}
          />
        )}
        {exercise.type === "FillInBlank" && (
          <FillInBlankView
            exercise={exercise as FillInBlankExercise}
            answered={answered}
            onAnswer={handleAnswer}
          />
        )}
      </div>

      {answered && result && !isIntro && (
        <div className={`popup-feedback ${wasCorrect ? "correct" : "incorrect"}`}>
          {wasCorrect
            ? `✓ Correct!${result.streak > 1 ? `  🔥 ${result.streak} streak` : ""}`
            : `✗ The answer was: "${result.word.term}"`}
        </div>
      )}

      <div className="popup-type-badge">
        {typeLabels[exercise.type] ?? exercise.type}
      </div>
    </div>
  );
};

const typeLabels: Record<string, string> = {
  Introduction: "New Word",
  MultipleChoice: "Meaning",
  TrueFalse: "True / False",
  DefinitionRecall: "Recall",
  ContextualGuess: "Context",
  FillInBlank: "Fill in Blank",
  SpellingCheck: "Spelling",
  SynonymMatch: "Synonym",
};

// ─── Exercise View Components ─────────────────────────────────────────────────

const IntroductionView: React.FC<{
  exercise: IntroductionExercise;
  onAcknowledge: () => void;
}> = ({ exercise, onAcknowledge }) => (
  <div className="intro-view">
    {exercise.isNewWord && <div className="new-badge">New Word</div>}
    <div className="intro-term">{exercise.term}</div>
    {exercise.phonetic && <div className="intro-phonetic">{exercise.phonetic}</div>}
    <div className="intro-pos">{exercise.partOfSpeech}</div>
    <div className="intro-definition">{exercise.definition}</div>
    {exercise.example && (
      <div className="intro-example">
        <span className="example-icon">💬</span>{exercise.example}
      </div>
    )}
    {exercise.synonyms.length > 0 && (
      <div className="intro-synonyms">
        {exercise.synonyms.map((s) => (
          <span key={s} className="synonym-chip">{s}</span>
        ))}
      </div>
    )}
    <button className="btn-primary" onClick={onAcknowledge}>Got it →</button>
  </div>
);

const MultipleChoiceView: React.FC<{
  exercise: MultipleChoiceExercise;
  answered: boolean;
  onAnswer: (correct: boolean, answer: string) => void;
}> = ({ exercise, answered, onAnswer }) => (
  <div className="mc-view">
    <div className="mc-question">{exercise.question}</div>
    {exercise.hint && <div className="mc-hint">({exercise.hint})</div>}
    <div className="mc-options">
      {exercise.options.map((opt, i) => (
        <button
          key={i}
          className={`mc-option ${answered ? (opt.isCorrect ? "opt-correct" : "opt-wrong-dim") : ""}`}
          onClick={() => !answered && onAnswer(opt.isCorrect, opt.text)}
          disabled={answered}
        >
          <span className="opt-letter">{String.fromCharCode(65 + i)}</span>
          {opt.text}
        </button>
      ))}
    </div>
  </div>
);

const TrueFalseView: React.FC<{
  exercise: TrueFalseExercise;
  answered: boolean;
  onAnswer: (correct: boolean, answer: string) => void;
}> = ({ exercise, answered, onAnswer }) => (
  <div className="tf-view">
    <div className="tf-term">{exercise.term}</div>
    <div className="tf-definition">{exercise.shownDefinition}</div>
    {answered && (
      <div className={`tf-explanation ${exercise.isCorrectDefinition ? "correct" : "incorrect"}`}>
        {exercise.explanation}
      </div>
    )}
    {!answered && (
      <div className="tf-buttons">
        <button className="btn-true" onClick={() => onAnswer(exercise.isCorrectDefinition, "true")}>
          ✓ True
        </button>
        <button className="btn-false" onClick={() => onAnswer(!exercise.isCorrectDefinition, "false")}>
          ✗ False
        </button>
      </div>
    )}
  </div>
);

const DefinitionRecallView: React.FC<{
  exercise: DefinitionRecallExercise;
  answered: boolean;
  onAnswer: (correct: boolean, answer: string) => void;
}> = ({ exercise, answered, onAnswer }) => (
  <div className="dr-view">
    <div className="dr-prompt">Which word matches this definition?</div>
    <div className="dr-definition">{exercise.definition}</div>
    <div className="dr-pos">({exercise.partOfSpeech})</div>
    <div className="mc-options">
      {exercise.options.map((opt, i) => (
        <button
          key={i}
          className={`mc-option ${answered ? (opt.isCorrect ? "opt-correct" : "opt-wrong-dim") : ""}`}
          onClick={() => !answered && onAnswer(opt.isCorrect, opt.text)}
          disabled={answered}
        >
          <span className="opt-letter">{String.fromCharCode(65 + i)}</span>
          {opt.text}
        </button>
      ))}
    </div>
  </div>
);

const ContextualGuessView: React.FC<{
  exercise: ContextualGuessExercise;
  answered: boolean;
  onAnswer: (correct: boolean, answer: string) => void;
}> = ({ exercise, answered, onAnswer }) => (
  <div className="cg-view">
    <div className="cg-prompt">
      Based on context, what does <strong>{exercise.term}</strong> mean?
    </div>
    <div className="cg-sentence">{exercise.contextSentence}</div>
    <div className="mc-options">
      {exercise.options.map((opt, i) => (
        <button
          key={i}
          className={`mc-option ${answered ? (opt.isCorrect ? "opt-correct" : "opt-wrong-dim") : ""}`}
          onClick={() => !answered && onAnswer(opt.isCorrect, opt.text)}
          disabled={answered}
        >
          <span className="opt-letter">{String.fromCharCode(65 + i)}</span>
          {opt.text}
        </button>
      ))}
    </div>
  </div>
);

const FillInBlankView: React.FC<{
  exercise: FillInBlankExercise;
  answered: boolean;
  onAnswer: (correct: boolean, answer: string) => void;
}> = ({ exercise, answered, onAnswer }) => (
  <div className="fib-view">
    <div className="fib-prompt">Fill in the blank:</div>
    <div className="fib-sentence">{exercise.sentence}</div>
    {exercise.hint && <div className="fib-hint">Hint: {exercise.hint}</div>}
    <div className="mc-options">
      {exercise.options.map((opt, i) => (
        <button
          key={i}
          className={`mc-option ${answered
            ? opt === exercise.answer ? "opt-correct" : "opt-wrong-dim"
            : ""}`}
          onClick={() => !answered && onAnswer(opt === exercise.answer, opt)}
          disabled={answered}
        >
          <span className="opt-letter">{String.fromCharCode(65 + i)}</span>
          {opt}
        </button>
      ))}
    </div>
  </div>
);
