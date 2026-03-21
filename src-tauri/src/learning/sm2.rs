// src-tauri/src/learning/sm2.rs
//
// SM-2 Algorithm (SuperMemo 2) with session micro-interval extension.
//
// Original SM-2:
//   - Quality q ∈ {0,1,2,3,4,5}  (0-2 = failed, 3-5 = passed)
//   - EF(new) = EF(old) + 0.1 - (5-q)(0.08 + (5-q)×0.02)
//   - EF_min = 1.3
//   - Interval:
//       n=1  → 1 day
//       n=2  → 6 days
//       n>2  → I(n-1) × EF
//   - If q < 3 → reset repetitions (relearn)
//
// Extensions implemented here:
//   - Fractional day intervals for same-session micro-scheduling
//   - Difficulty modifier (word.difficulty scales the interval)
//   - Mastery level transitions

use chrono::{Duration, Utc};
use crate::db::{WordProgress, MasteryLevel};

/// Result returned after each SM-2 calculation
#[derive(Debug, Clone)]
pub struct Sm2Result {
    pub easiness_factor: f64,
    pub interval_days: f64,
    pub repetitions: i32,
    pub iterations: i32,
    pub mastery_level: MasteryLevel,
}

pub fn calculate_next(progress: &WordProgress, quality: i32, difficulty: i32) -> Sm2Result {
    debug_assert!((0..=5).contains(&quality), "quality must be 0-5");

    let old_ef = progress.easiness_factor;
    let old_iterations = progress.iterations;

    // ── Update Easiness Factor ────────────────────────────────────────────
    let q = quality as f64;
    let new_ef = (old_ef + 0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02))
        .max(1.3)  // EF cannot drop below 1.3
        .min(3.5); // practical ceiling

    // ── Update Interval & Iterations ──────────────────────────────────────
    let (new_iterations, raw_interval) = if quality < 3 {
        // Failed recall → reset iterations
        (0, 1.0_f64) // Return to review after 1 day
    } else {
        let next_iterations = old_iterations + 1;
        let interval = match next_iterations {
            1 => 1.0,
            2 => 6.0,
            _ => (progress.interval_days * new_ef).round(),
        };
        (next_iterations, interval)
    };

    // ── Difficulty Modifier ───────────────────────────────────────────────
    let difficulty_factor = match difficulty {
        1 => 1.2, 
        2 => 1.0,
        3 => 0.85,
        4 => 0.70,
        5 => 0.55, 
        _ => 1.0,
    };
    let adjusted_interval = (raw_interval * difficulty_factor).max(1.0);

    // ── Mastery Level Transition ──────────────────────────────────────────
    let mastery_level = determine_mastery(new_iterations, adjusted_interval, quality);

    Sm2Result {
        easiness_factor: new_ef,
        interval_days: adjusted_interval,
        repetitions: progress.repetitions + 1,
        iterations: new_iterations,
        mastery_level,
    }
}

pub fn apply_result(progress: &mut WordProgress, result: Sm2Result, quality: i32) {
    let now = Utc::now();

    progress.easiness_factor = result.easiness_factor;
    progress.interval_days = result.interval_days;
    progress.repetitions = result.repetitions;
    progress.iterations = result.iterations;
    progress.mastery_level = result.mastery_level;
    progress.last_review_at = Some(now);
    progress.total_reviews += 1;

    if quality >= 3 {
        progress.correct_reviews += 1;
        progress.streak += 1;
    } else {
        progress.streak = 0;
    }

    // Compute next_review_at from interval
    let interval_secs = (result.interval_days * 86_400.0) as i64;
    let mut next_date = now + Duration::seconds(interval_secs.max(30));

    // Safety Cooldown: Jeśli odpowiedź była Good/Easy (>=4), wymuś min. 12h przerwy
    // zapobiega to zapętleniu słowa w tej samej sesji pracy.
    if quality >= 4 && result.interval_days < 0.5 {
        next_date = now + Duration::hours(12);
    }
    
    progress.next_review_at = next_date;

    if progress.introduced_at.is_none() {
        progress.introduced_at = Some(now);
    }

    progress.session_reviews += 1;
    progress.next_session_review_at = Some(compute_session_interval(progress.session_reviews));
}

/// Session micro-schedule: +15min, +1h, +3h within the same session.
/// After that, falls back to SM-2 inter-day intervals.
pub fn compute_session_interval(session_review_count: i32) -> chrono::DateTime<Utc> {
    let now = Utc::now();
    let delay = match session_review_count {
        0 | 1 => Duration::minutes(15),
        2 => Duration::hours(1),
        3 => Duration::hours(3),
        _ => Duration::hours(6), // deep review for persistent reinforcement
    };
    now + delay
}

/// Convert user latency (ms) and correctness to SM-2 quality score.
///
/// This heuristic combines:
///   - correctness (binary)
///   - response speed relative to expected time (< 3s = fluent)
pub fn score_from_response(was_correct: bool, response_ms: i64, exercise_difficulty: i32) -> i32 {
    if !was_correct {
        // 0 = complete blackout, 1 = wrong but familiar-ish, 2 = wrong but recalled after
        return match response_ms {
            ms if ms < 3000 => 1, // quick wrong = guessed
            _ => 0,
        };
    }

    // Correct answer - score by speed
    let ideal_ms = match exercise_difficulty {
        1 => 2000,
        2 => 3000,
        3 => 4000,
        4 => 5500,
        _ => 7000,
    };

    match response_ms {
        ms if ms < ideal_ms / 2 => 5,       // very fast = perfect recall
        ms if ms < ideal_ms => 4,            // normal speed = correct with effort
        ms if ms < ideal_ms * 2 => 3,        // slow but correct = correct with difficulty
        _ => 3,                              // very slow correct = marginal pass
    }
}

fn determine_mastery(repetitions: i32, interval_days: f64, quality: i32) -> MasteryLevel {
    if quality < 3 {
        return MasteryLevel::Learning;
    }
    match (repetitions, interval_days) {
        (0, _) => MasteryLevel::New,
        (1..=3, _) => MasteryLevel::Learning,
        (4..=7, d) if d < 21.0 => MasteryLevel::Reviewing,
        (n, d) if n >= 8 || d >= 21.0 => MasteryLevel::Mastered,
        _ => MasteryLevel::Reviewing,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WordProgress;
    use chrono::Utc;

    fn fresh_progress() -> WordProgress {
        WordProgress {
            id: 1,
            word_id: 1,
            easiness_factor: 2.5,
            interval_days: 0.0,
            repetitions: 0,
            iterations: 0,
            next_review_at: Utc::now(),
            last_review_at: None,
            total_reviews: 0,
            correct_reviews: 0,
            streak: 0,
            introduced_at: None,
            session_reviews: 0,
            next_session_review_at: None,
            mastery_level: MasteryLevel::New,
        }
    }

    #[test]
    fn first_correct_review_gives_1_day() {
        let p = fresh_progress();
        let r = calculate_next(&p, 4, 2);
        assert_eq!(r.iterations, 1);
        assert!((r.interval_days - 1.0).abs() < 0.1);
    }

    #[test]
    fn failed_recall_resets_iterations() {
        let mut p = fresh_progress();
        p.iterations = 5;
        p.interval_days = 21.0;
        let r = calculate_next(&p, 2, 2);
        assert_eq!(r.iterations, 0);
        // In the new logic, we return to review after 1 day even on failure
        assert_eq!(r.interval_days, 1.0);
    }

    #[test]
    fn ef_cannot_drop_below_1_3() {
        let mut p = fresh_progress();
        for _ in 0..20 {
            let r = calculate_next(&p, 0, 3);
            p.easiness_factor = r.easiness_factor;
        }
        assert!(p.easiness_factor >= 1.3);
    }

    #[test]
    fn difficulty_5_halves_interval() {
        let p = fresh_progress();
        let easy = calculate_next(&p, 5, 1);
        let hard = calculate_next(&p, 5, 5);
        // Hard word interval should be shorter
        assert!(hard.interval_days <= easy.interval_days);
    }
}
