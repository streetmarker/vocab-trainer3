// src/types/index.ts

export type PartOfSpeech =
  | "noun" | "verb" | "adjective" | "adverb" | "pronoun"
  | "preposition" | "conjunction" | "interjection" | "phrase";

export type MasteryLevel = "new" | "learning" | "reviewing" | "mastered";

export type ExerciseType =
  | "introduction" | "multiple_choice" | "fill_in_blank" | "contextual_guess"
  | "spelling_check" | "synonym_match" | "definition_recall" | "true_false";

export interface Word {
  id: number;
  term: string;
  definition: string;
  definitionPl?: string;       // Polish translation/explanation
  partOfSpeech: PartOfSpeech;
  phonetic?: string;
  examples: string[];
  synonyms: string[];
  antonyms: string[];
  tags: string[];
  difficulty: 1 | 2 | 3 | 4 | 5;
  createdAt: string;
  isActive: boolean;
  sentencePl?: string;         // Example sentence in Polish — word bolded on flashcard front
  sentenceEn?: string;         // Example sentence in English — word bolded on flashcard back
  category?: string;
}

export interface WordWithProgress extends Word {
  masteryLevel: MasteryLevel;
  repetitions: number;
  intervalDays: number;
  easeFactor: number;
  streak: number;
  totalReviews: number;
  nextReviewAt?: string;
  lastReviewAt?: string;
  reviewStatus: "never" | "overdue" | "today" | "future";
}

export interface WordProgress {
  id: number;
  wordId: number;
  easinessFactor: number;
  intervalDays: number;
  repetitions: number;
  nextReviewAt: string;
  lastReviewAt?: string;
  totalReviews: number;
  correctReviews: number;
  streak: number;
  introducedAt?: string;
  sessionReviews: number;
  nextSessionReviewAt?: string;
  masteryLevel: MasteryLevel;
}

// ─── Exercise Payloads ────────────────────────────────────────────────────────

export interface McOption {
  text: string;
  isCorrect: boolean;
}

export interface IntroductionExercise {
  type: "Introduction";
  wordId: number;
  term: string;
  phonetic?: string;
  partOfSpeech: string;
  definition: string;
  definitionPl?: string;
  example?: string;
  synonyms: string[];
  isNewWord: boolean;
}

export interface MultipleChoiceExercise {
  type: "MultipleChoice";
  wordId: number;
  term: string;
  question: string;
  options: McOption[];
  correctIndex: number;
  hint?: string;
}

export interface FillInBlankExercise {
  type: "FillInBlank";
  wordId: number;
  sentence: string;
  answer: string;
  hint?: string;
  options: string[];
}

export interface ContextualGuessExercise {
  type: "ContextualGuess";
  wordId: number;
  term: string;
  contextSentence: string;
  options: McOption[];
  correctIndex: number;
}

export interface SpellingCheckExercise {
  type: "SpellingCheck";
  wordId: number;
  definition: string;
  answer: string;
  phonetic?: string;
  hint: string;
}

export interface SynonymMatchExercise {
  type: "SynonymMatch";
  wordId: number;
  term: string;
  options: McOption[];
  correctIndices: number[];
  question: string;
}

export interface DefinitionRecallExercise {
  type: "DefinitionRecall";
  wordId: number;
  definition: string;
  partOfSpeech: string;
  answer: string;
  options: McOption[];
  correctIndex: number;
}

export interface TrueFalseExercise {
  type: "TrueFalse";
  wordId: number;
  term: string;
  shownDefinition: string;
  isCorrectDefinition: boolean;
  explanation: string;
}

export type Exercise =
  | IntroductionExercise
  | MultipleChoiceExercise
  | FillInBlankExercise
  | ContextualGuessExercise
  | SpellingCheckExercise
  | SynonymMatchExercise
  | DefinitionRecallExercise
  | TrueFalseExercise;

// ─── Answer / Result ──────────────────────────────────────────────────────────

export interface SubmitAnswerPayload {
  wordId: number;
  wasCorrect: boolean;
  responseTimeMs: number;
  userAnswer?: string;
  exerciseType: ExerciseType;
}

export interface AnswerResult {
  quality: number;
  wasCorrect: boolean;
  newIntervalDays: number;
  masteryLevel: string;
  nextReviewAt: string;
  streak: number;
  word: Word;
}

// ─── Statistics ───────────────────────────────────────────────────────────────

export interface OverallStats {
  totalWords: number;
  masteredWords: number;
  totalExercises: number;
  correctAnswers: number;
  accuracyPercent: number;
  currentStreak: number;
}

export interface DailyStats {
  date: string;
  exercisesCompleted: number;
  correctAnswers: number;
  wordsReviewed: number;
  wordsMastered: number;
  totalTimeMs: number;
  streakDays: number;
}

export interface ActivityDay {
  date: string;
  count: number;
  correct: number;
}

// ─── Scheduler ────────────────────────────────────────────────────────────────

export interface SchedulerStatus {
  isReady: boolean;
  conditions: {
    userIsIdle: boolean;
    noFullscreen: boolean;
    enoughTimeSinceLast: boolean;
    withinWorkHours: boolean;
    notPaused: boolean;
    hasDueExercises: boolean;
    underDailyLimit: boolean;
  };
  blockedReason?: string;
}

// ─── UI State ─────────────────────────────────────────────────────────────────

export type AppRoute = "dashboard" | "vocab" | "settings" | "popup";

export interface ExerciseSession {
  currentExercise: Exercise | null;
  startedAt: number;
  sessionId: string;
}

export type DifficultyLabel = "Początkujący" | "Podstawowy" | "Średni" | "Zaawansowany" | "Ekspert";

export const DIFFICULTY_LABELS: Record<number, DifficultyLabel> = {
  1: "Początkujący",
  2: "Podstawowy",
  3: "Średni",
  4: "Zaawansowany",
  5: "Ekspert",
};

export const DIFFICULTY_COLORS: Record<number, string> = {
  1: "#22c55e",
  2: "#84cc16",
  3: "#eab308",
  4: "#f97316",
  5: "#ef4444",
};

export const MASTERY_COLORS: Record<MasteryLevel, string> = {
  new: "#6b7280",
  learning: "#3b82f6",
  reviewing: "#f59e0b",
  mastered: "#10b981",
};

export const MASTERY_LABELS: Record<MasteryLevel, string> = {
  new: "Nowe",
  learning: "W nauce",
  reviewing: "Powtórka",
  mastered: "Opanowane",
};

export const PART_OF_SPEECH_LABELS: Record<string, string> = {
  noun: "rzeczownik",
  verb: "czasownik",
  adjective: "przymiotnik",
  adverb: "przysłówek",
  pronoun: "zaimek",
  preposition: "przyimek",
  conjunction: "spójnik",
  interjection: "wykrzyknik",
  phrase: "wyrażenie",
};

// ─── Mentor Types ─────────────────────────────────────────────────────────────

export interface MentorTip {
  wordId: number;
  term: string;
  mnemonic: string;
  businessStory: string;
  deepDiveExplanation: string;
  generatedAt: string;
}

export type MentorPayload = Record<number, MentorTip>;
