// src-tauri/src/learning/mod.rs

pub mod sm2;
pub mod exercise_generator;
pub mod scheduler;

use anyhow::Result;
use chrono::Utc;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{Database, ExerciseHistory, Word, WordProgress};
use exercise_generator::{Exercise, ExerciseGenerator};
use sm2::{apply_result, calculate_next, score_from_response};

// ─── Learning Engine ──────────────────────────────────────────────────────────

pub struct LearningEngine {
    db: Arc<Database>,
}

impl LearningEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn build_exercise(&self, word_id: i64) -> Result<Exercise> {
        let word = self
            .db
            .get_word_by_id(word_id)?
            .ok_or_else(|| anyhow::anyhow!("Word {} not found", word_id))?;
        let progress = self.db.get_or_create_progress(word_id)?;
        let distractors = self.db.get_words_for_distractor(word_id, 5)?;
        let mut rng = SmallRng::from_entropy();
        ExerciseGenerator::generate(&word, &progress, &distractors, &mut rng)
    }

    pub fn process_answer(
        &self,
        word_id: i64,
        was_correct: bool,
        response_time_ms: i64,
        user_answer: Option<String>,
        exercise_type: crate::db::ExerciseType,
        session_id: &str,
    ) -> Result<AnswerResult> {
        let word = self
            .db
            .get_word_by_id(word_id)?
            .ok_or_else(|| anyhow::anyhow!("Word {} not found", word_id))?;
        let mut progress = self.db.get_or_create_progress(word_id)?;

        let quality = score_from_response(was_correct, response_time_ms, word.difficulty);
        let sm2_result = calculate_next(&progress, quality, word.difficulty);
        apply_result(&mut progress, sm2_result, quality);

        self.db.update_progress(&progress)?;

        let history = ExerciseHistory {
            id: 0,
            word_id,
            exercise_type,
            response_time_ms,
            quality,
            was_correct,
            user_answer,
            session_id: session_id.to_string(),
            completed_at: Utc::now(),
        };
        self.db.record_exercise(&history)?;

        Ok(AnswerResult {
            quality,
            was_correct,
            new_interval_days: progress.interval_days,
            new_ef: progress.easiness_factor,
            mastery_level: progress.mastery_level.as_str().to_string(),
            next_review_at: progress.next_review_at.to_rfc3339(),
            streak: progress.streak,
            word,
        })
    }

    pub fn start_session(&self) -> Result<Option<(Word, Exercise)>> {
        if let Some((word, mut progress)) = self.db.get_session_word()? {
            progress.introduced_at = Some(Utc::now());
            progress.session_reviews = 0;
            progress.next_session_review_at = Some(sm2::compute_session_interval(0));
            self.db.update_progress(&progress)?;

            let distractors = self.db.get_words_for_distractor(word.id, 5)?;
            let mut rng = SmallRng::from_entropy();
            let exercise = ExerciseGenerator::generate_typed(
                crate::db::ExerciseType::Introduction,
                &word,
                &progress,
                &distractors,
                &mut rng,
            )?;
            Ok(Some((word, exercise)))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerResult {
    pub quality: i32,
    pub was_correct: bool,
    pub new_interval_days: f64,
    pub new_ef: f64,
    pub mastery_level: String,
    pub next_review_at: String,
    pub streak: i32,
    pub word: Word,
}

// ─── Difficulty Adjuster ──────────────────────────────────────────────────────

pub struct DifficultyAdjuster;

impl DifficultyAdjuster {
    pub fn adjust(word: &Word, progress: &WordProgress) -> Option<i32> {
        if progress.total_reviews < 5 {
            return None;
        }
        let accuracy = progress.correct_reviews as f64 / progress.total_reviews as f64;
        let current = word.difficulty;
        let new_difficulty = if accuracy > 0.90 && current > 1 {
            current - 1
        } else if accuracy < 0.50 && current < 5 {
            current + 1
        } else {
            current
        };
        if new_difficulty != current { Some(new_difficulty) } else { None }
    }

    pub fn label(difficulty: i32) -> &'static str {
        match difficulty {
            1 => "Beginner",
            2 => "Elementary",
            3 => "Intermediate",
            4 => "Advanced",
            5 => "Expert",
            _ => "Unknown",
        }
    }
}

// ─── Progress Tracker ─────────────────────────────────────────────────────────

pub struct ProgressTracker {
    db: Arc<Database>,
}

impl ProgressTracker {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn get_overall_stats(&self) -> Result<serde_json::Value> {
        self.db.get_overall_stats()
    }

    pub fn get_daily_stats(&self, days: i32) -> Result<Vec<crate::db::DailyStats>> {
        self.db.get_daily_stats(days)
    }

    pub fn get_activity_grid(&self) -> Result<Vec<serde_json::Value>> {
        let stats = self.db.get_daily_stats(365)?;
        Ok(stats.into_iter().map(|s| serde_json::json!({
            "date": s.date,
            "count": s.exercises_completed,
            "correct": s.correct_answers,
        })).collect())
    }

    pub fn get_struggling_words(&self) -> Result<Vec<Word>> {
        self.db.get_all_words()
    }
}

// ─── Session ID ───────────────────────────────────────────────────────────────

pub fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}
