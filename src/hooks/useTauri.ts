// src/hooks/useTauri.ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import type {
  Word, Exercise, AnswerResult, OverallStats,
  DailyStats, ActivityDay, SchedulerStatus,
} from "../types";

export const api = {
  getExercise: (wordId: number): Promise<Exercise> =>
    invoke("get_exercise", { wordId }),

  submitAnswer: (payload: {
    wordId: number;
    wasCorrect: boolean;
    responseTimeMs: number;
    userAnswer?: string;
    exerciseType: string;
  }): Promise<AnswerResult> => invoke("submit_answer", payload),

  startSession: (): Promise<{ word: Word; exercise: Exercise } | null> =>
    invoke("start_session"),

  getWords: (): Promise<Word[]> =>
    invoke("get_words"),

  getSrsOverview: (): Promise<import("./useTauri").SrsOverview> =>
    invoke("get_srs_overview"),

  addWord: (word: {
    term: string;
    definition: string;
    definitionPl?: string;
    partOfSpeech: string;
    phonetic?: string;
    examples: string[];
    synonyms: string[];
    antonyms: string[];
    tags: string[];
    difficulty: number;
    sentencePl?: string;
    sentenceEn?: string;
  }): Promise<number> => invoke("add_word", word),

  deleteWord: (wordId: number): Promise<void> =>
    invoke("delete_word", { wordId }),

  clearWords: (): Promise<number> =>
    invoke("clear_words"),

  getOverallStats: (): Promise<OverallStats> =>
    invoke("get_overall_stats"),

  getDailyStats: (days: number): Promise<DailyStats[]> =>
    invoke("get_daily_stats", { days }),

  getActivityGrid: (): Promise<ActivityDay[]> =>
    invoke("get_activity_grid"),

  getSchedulerStatus: (): Promise<SchedulerStatus> =>
    invoke("get_scheduler_status"),

  setSchedulerPaused: (paused: boolean): Promise<void> =>
    invoke("set_scheduler_paused", { paused }),

  seedSampleWords: (): Promise<number> =>
    invoke("seed_sample_words"),

  getSettings: (): Promise<{
    exercisesPerDay: number;
    idleThresholdSecs: number;
    minGapMinutes: number;
    autostart: boolean;
    showSessionWord: boolean;
    soundEffects: boolean;
    workHoursOnly: boolean;
    workHoursStart: string;
    workHoursEnd: string;
  }> => invoke("get_settings"),

  saveSettings: (settings: {
    exercisesPerDay: number;
    idleThresholdSecs: number;
    minGapMinutes: number;
    autostart: boolean;
    showSessionWord: boolean;
    soundEffects: boolean;
    workHoursOnly: boolean;
    workHoursStart: string;
    workHoursEnd: string;
  }): Promise<void> => invoke("save_settings", { settings }),

  hidePopup: (): Promise<void> => invoke("hide_popup"),

  getPopupExercise: (): Promise<import("../types").Exercise | null> =>
    invoke("get_popup_exercise"),

  triggerPopup: (): Promise<boolean> =>
    invoke("trigger_popup"),

  getCurrentWord: (): Promise<import("../types").Word | null> =>
    invoke("get_current_word"),

  taskNotificationDone: (wordId: number): Promise<void> =>
    invoke("task_notification_done", { wordId }),

  taskNotificationLater: (wordId: number): Promise<void> =>
    invoke("task_notification_later", { wordId }),

  taskNotificationKnown: (wordId: number): Promise<void> =>
    invoke("task_notification_known", { wordId }),

  /** Primary SRS command — grades a flashcard and returns the next word. */
  srsAnswer: (wordId: number, grade: "again" | "hard" | "good" | "easy"): Promise<{
    wordId:            number;
    grade:             string;
    newMastery:        string;
    newIntervalDays:   number;
    newEasiness:       number;
    streak:            number;
    nextReviewLabel:   string;
    nextReviewAt:      string;
    nextWordId:        number | null;
    nextTermPl:        string | null;
    nextTermEn:        string | null;
    nextPartOfSpeech:  string | null;
    nextPhonetic:      string | null;
    nextSentencePl:    string | null;
    nextSentenceEn:    string | null;
  }> => invoke("srs_answer", { wordId, grade }),

  flashcardAnswer: (wordId: number, decision: "known" | "practice"): Promise<{
    wordId: number; decision: string; newMastery: string;
    newIntervalDays: number; streak: number;
    nextWordId: number | null; nextTermPl: string | null;
    nextTermEn: string | null; nextPartOfSpeech: string | null;
  }> => invoke("flashcard_answer", { wordId, decision }),

  importWordsFromJson: (json: string): Promise<{
    added: number;
    skipped: number;
    warnings: string[];
  }> => invoke("import_words_from_json", { json }),

  getStrugglingWords: (limit: number): Promise<Word[]> =>
    invoke("get_struggling_words", { limit }),

  getMentorTips: (): Promise<Record<number, import("../types").MentorTip>> =>
    invoke("get_mentor_tips"),

  saveMentorTips: (tips: Record<number, import("../types").MentorTip>): Promise<void> =>
    invoke("save_mentor_tips", { tips }),
};

export function useTauriEvent<T = unknown>(
  event: string,
  handler: (payload: T) => void
) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    listen<T>(event, (e) => handlerRef.current(e.payload)).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [event]);
}

// ─── SRS Overview ─────────────────────────────────────────────────────────────

export type SrsReviewStatus = "overdue" | "today" | "future" | "never";
export type SrsMastery      = "new" | "learning" | "reviewing" | "mastered";

export interface WordWithProgress {
  id:            number;
  term:          string;
  definition:    string;
  definitionPl?: string;
  partOfSpeech:  string;
  phonetic?:     string;
  difficulty:    number;
  tags:          string[];
  sentencePl?:   string;
  sentenceEn?:   string;
  // SRS
  masteryLevel:  SrsMastery;
  repetitions:   number;
  intervalDays:  number;
  easeFactor:    number;
  streak:        number;
  totalReviews:  number;
  nextReviewAt?: string;
  lastReviewAt?: string;
  reviewStatus:  SrsReviewStatus;
}

export interface SrsTodayStats {
  dueToday: number;
  newWords: number;
  learning: number;
  reviewing: number;
  mastered:  number;
  total:     number;
}

export interface SrsOverview {
  today: SrsTodayStats;
  words: WordWithProgress[];
}
