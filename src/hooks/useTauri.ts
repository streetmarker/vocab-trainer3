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
  }): Promise<number> => invoke("add_word", word),

  deleteWord: (wordId: number): Promise<void> =>
    invoke("delete_word", { wordId }),

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
