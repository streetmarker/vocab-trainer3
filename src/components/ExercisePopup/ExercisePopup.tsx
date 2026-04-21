// src/components/ExercisePopup/ExercisePopup.tsx
import React, { useState, useCallback, useRef } from "react";
import type {
  Exercise, IntroductionExercise, MultipleChoiceExercise,
  FillInBlankExercise, TrueFalseExercise, DefinitionRecallExercise,
  ContextualGuessExercise, AnswerResult,
} from "../../types";
import { PART_OF_SPEECH_LABELS } from "../../types";
import { api } from "../../hooks/useTauri";
import { formatReviewDate } from "../../utils/date";
import TtsPlayer from "../TtsPlayer";
import "./ExercisePopup.css";

interface Props {
  exercise: Exercise;
  onComplete: (result: AnswerResult) => void;
  onDismiss: () => void;
}

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
      console.error("Błąd zapisu odpowiedzi:", err);
    }
  }, [answered, exercise, onComplete]);

  const handleDismiss = () => {
    setIsExiting(true);
    setTimeout(onDismiss, 300);
  };

  return (
    <div className={`popup-root ${isExiting ? "exiting" : "entering"}`}>
      <div className="popup-header">
        <div className="popup-logo">
          <span className="logo-icon">📚</span>
          <span className="logo-text">VocabTrainer</span>
        </div>
        <button className="popup-dismiss" onClick={handleDismiss} aria-label="Zamknij">✕</button>
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

      {answered && result && exercise.type !== "Introduction" && (
        <div className={`popup-feedback ${wasCorrect ? "correct" : "incorrect"}`}>
          <div className="feedback-main">
            {wasCorrect
              ? `✓ Poprawnie!${result.streak > 1 ? `  🔥 Seria: ${result.streak}` : ""}`
              : `✗ Odpowiedź: "${result.word.term}"`}
          </div>
          <div className="feedback-next">
            Następna powtórka: <strong>{formatReviewDate(result.nextReviewAt)}</strong>
          </div>
        </div>
      )}

      <div className="popup-type-badge">
        {typeLabels[exercise.type] ?? exercise.type}
      </div>
    </div>
  );
};

const typeLabels: Record<string, string> = {
  Introduction:     "Nowe słowo",
  MultipleChoice:   "Wybór znaczenia",
  TrueFalse:        "Prawda / Fałsz",
  DefinitionRecall: "Rozpoznaj słowo",
  ContextualGuess:  "Z kontekstu",
  FillInBlank:      "Uzupełnij lukę",
  SpellingCheck:    "Pisownia",
  SynonymMatch:     "Synonim",
};

// ─── Exercise Views ───────────────────────────────────────────────────────────

const IntroductionView: React.FC<{
  exercise: IntroductionExercise;
  onAcknowledge: () => void;
}> = ({ exercise, onAcknowledge }) => (
  <div className="intro-view">
    {exercise.isNewWord && <div className="new-badge">Nowe słowo</div>}
    <div className="intro-term">{exercise.term}</div>
    {exercise.phonetic && <div className="intro-phonetic">{exercise.phonetic}</div>}
    <div className="intro-pos">{PART_OF_SPEECH_LABELS[exercise.partOfSpeech] ?? exercise.partOfSpeech}</div>

    <div className="intro-definition">{exercise.definition}</div>

    <TtsPlayer 
      term={exercise.term} 
      exampleEn={exercise.example ?? ""} 
      autoPlay={false}
    />

    {exercise.definitionPl && (
      <div className="intro-definition-pl">
        <span className="pl-flag">🇵🇱</span>
        {exercise.definitionPl}
      </div>
    )}

    {exercise.example && (
      <div className="intro-example">
        <span className="example-icon">💬</span>
        <em>{exercise.example}</em>
      </div>
    )}

    {exercise.synonyms.length > 0 && (
      <div className="intro-synonyms">
        <span className="synonyms-label">Synonimy: </span>
        {exercise.synonyms.map((s) => (
          <span key={s} className="synonym-chip">{s}</span>
        ))}
      </div>
    )}

    <button className="btn-primary" onClick={onAcknowledge}>Rozumiem →</button>
  </div>
);

const MultipleChoiceView: React.FC<{
  exercise: MultipleChoiceExercise;
  answered: boolean;
  onAnswer: (correct: boolean, answer: string) => void;
}> = ({ exercise, answered, onAnswer }) => (
  <div className="mc-view">
    <div className="mc-question">Jakie jest znaczenie słowa?</div>
    <div className="mc-term">{exercise.term}</div>
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
    <div className="dr-prompt">Czy ta definicja pasuje do słowa?</div>
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
          ✓ Prawda
        </button>
        <button className="btn-false" onClick={() => onAnswer(!exercise.isCorrectDefinition, "false")}>
          ✗ Fałsz
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
    <div className="dr-prompt">Które słowo odpowiada tej definicji?</div>
    <div className="dr-definition">{exercise.definition}</div>
    <div className="dr-pos">({PART_OF_SPEECH_LABELS[exercise.partOfSpeech] ?? exercise.partOfSpeech})</div>
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
      Co oznacza słowo <strong>{exercise.term}</strong> w tym zdaniu?
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
    <div className="fib-prompt">Uzupełnij brakujące słowo:</div>
    <div className="fib-sentence">{exercise.sentence}</div>
    {exercise.hint && <div className="fib-hint">Podpowiedź: {exercise.hint}</div>}
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
