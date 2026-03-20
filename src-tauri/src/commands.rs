// src-tauri/src/commands.rs
//
// All Tauri commands exposed to the frontend via invoke().

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

use std::sync::Arc;
use tauri::State;

use crate::db::{Database, Word, ExerciseType};
use crate::learning::{LearningEngine, ProgressTracker, AnswerResult};
use crate::learning::exercise_generator::Exercise;
use crate::learning::scheduler::Scheduler;

// ─── App State (dependency injection via Tauri) ───────────────────────────────

pub struct AppState {
    pub db: Arc<Database>,
    pub engine: Arc<LearningEngine>,
    pub tracker: Arc<ProgressTracker>,
    pub scheduler: Arc<Scheduler>,
    pub data_dir: PathBuf,
    /// Word queued for the popup window — set before showing the window
    pub pending_word_id: Mutex<Option<i64>>,
}

// ─── Exercise Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_exercise(
    word_id: i64,
    state: State<'_, AppState>,
) -> Result<Exercise, String> {
    state.engine.build_exercise(word_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn submit_answer(
    word_id: i64,
    was_correct: bool,
    response_time_ms: i64,
    user_answer: Option<String>,
    exercise_type: String,
    state: State<'_, AppState>,
) -> Result<AnswerResult, String> {
    let ex_type = ExerciseType::from_str(&exercise_type);
    let session_id = state.scheduler.session_id();
    state
        .engine
        .process_answer(word_id, was_correct, response_time_ms, user_answer, ex_type, &session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_session(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    match state.engine.start_session() {
        Ok(Some((word, exercise))) => Ok(Some(serde_json::json!({
            "word": word,
            "exercise": exercise,
        }))),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

// ─── Word / Vocabulary Commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn get_words(state: State<'_, AppState>) -> Result<Vec<Word>, String> {
    state.db.get_all_words().map_err(|e| e.to_string())
}

// ─── SRS Overview ─────────────────────────────────────────────────────────────

/// Single word enriched with its SRS progress for the overview screen.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordWithProgress {
    // Core word fields (mirrors Word struct for frontend)
    pub id:             i64,
    pub term:           String,
    pub definition:     String,
    pub definition_pl:  Option<String>,
    pub part_of_speech: String,
    pub phonetic:       Option<String>,
    pub difficulty:     i32,
    pub tags:           Vec<String>,
    pub sentence_pl:    Option<String>,
    pub sentence_en:    Option<String>,
    // SRS progress (None if word has never been reviewed)
    pub mastery_level:  String,
    pub repetitions:    i32,
    pub interval_days:  f64,
    pub ease_factor:    f64,
    pub streak:         i32,
    pub total_reviews:  i32,
    pub next_review_at: Option<String>,
    pub last_review_at: Option<String>,
    pub review_status:  String,
}

/// Aggregate counts used by the "Stan nauki dziś" panel.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SrsTodayStats {
    pub due_today:  usize,   // next_review_at <= now
    pub new_words:  usize,   // mastery_level = "new" (never reviewed)
    pub learning:   usize,   // mastery_level = "learning"
    pub reviewing:  usize,   // mastery_level = "reviewing"
    pub mastered:   usize,   // mastery_level = "mastered"
    pub total:      usize,
}

/// Full SRS overview — returned in one call to avoid multiple round-trips.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SrsOverview {
    pub today:    SrsTodayStats,
    pub words:    Vec<WordWithProgress>,
}

/// Returns all active words joined with their SRS progress (or defaults for new words).
/// The frontend uses this to render the grouped word list with SRS badges.
#[tauri::command]
pub async fn get_srs_overview(state: State<'_, AppState>) -> Result<SrsOverview, String> {
    let now = chrono::Utc::now();
    let today_str = now.format("%Y-%m-%d").to_string();

    let words: Vec<Word> = state.db.get_all_words().map_err(|e| e.to_string())?;

    let mut enriched: Vec<WordWithProgress> = Vec::with_capacity(words.len());

    for word in &words {
        // get_or_create_progress is cheap for new words (INSERT OR IGNORE pattern)
        let p = state.db.get_or_create_progress(word.id).map_err(|e| e.to_string())?;

        let next_iso = Some(p.next_review_at.to_rfc3339());
        let last_iso = p.last_review_at.map(|t| t.to_rfc3339());

        let review_status = if p.total_reviews == 0 {
            "never".to_string()
        } else if p.next_review_at <= now {
            "overdue".to_string()
        } else {
            let next_date = p.next_review_at.format("%Y-%m-%d").to_string();
            if next_date == today_str { "today".to_string() } else { "future".to_string() }
        };

        enriched.push(WordWithProgress {
            id:             word.id,
            term:           word.term.clone(),
            definition:     word.definition.clone(),
            definition_pl:  word.definition_pl.clone(),
            part_of_speech: word.part_of_speech.clone(),
            phonetic:       word.phonetic.clone(),
            difficulty:     word.difficulty,
            tags:           word.tags.clone(),
            sentence_pl:    word.sentence_pl.clone(),
            sentence_en:    word.sentence_en.clone(),
            mastery_level:  p.mastery_level.as_str().to_string(),
            repetitions:    p.repetitions,
            interval_days:  p.interval_days,
            ease_factor:    p.easiness_factor,
            streak:         p.streak,
            total_reviews:  p.total_reviews,
            next_review_at: next_iso,
            last_review_at: last_iso,
            review_status,
        });
    }

    let today = SrsTodayStats {
        due_today: enriched.iter().filter(|w| w.review_status == "overdue" || w.review_status == "today").count(),
        new_words: enriched.iter().filter(|w| w.mastery_level == "new").count(),
        learning:  enriched.iter().filter(|w| w.mastery_level == "learning").count(),
        reviewing: enriched.iter().filter(|w| w.mastery_level == "reviewing").count(),
        mastered:  enriched.iter().filter(|w| w.mastery_level == "mastered").count(),
        total:     enriched.len(),
    };

    Ok(SrsOverview { today, words: enriched })
}

#[tauri::command]
pub async fn add_word(
    term: String,
    definition: String,
    definition_pl: Option<String>,
    part_of_speech: String,
    phonetic: Option<String>,
    examples: Vec<String>,
    synonyms: Vec<String>,
    antonyms: Vec<String>,
    tags: Vec<String>,
    difficulty: i32,
    sentence_pl: Option<String>,
    sentence_en: Option<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let word = Word {
        id: 0,
        term,
        definition,
        definition_pl,
        part_of_speech,
        phonetic,
        examples,
        synonyms,
        antonyms,
        tags,
        difficulty: difficulty.clamp(1, 5),
        created_at: chrono::Utc::now(),
        is_active: true,
        sentence_pl,
        sentence_en,
    };
    state.db.insert_word(&word).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_word(word_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    state.db.delete_word(word_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_words(state: State<'_, AppState>) -> Result<usize, String> {
    let deleted = state.db.clear_all_words().map_err(|e| e.to_string())?;
    log::info!("clear_words: permanently deleted {} words (cascade: progress + history)", deleted);
    Ok(deleted)
}

// ─── Stats / Progress Commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn get_overall_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.tracker.get_overall_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_daily_stats(
    days: i32,
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::DailyStats>, String> {
    state.tracker.get_daily_stats(days).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_activity_grid(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    state.tracker.get_activity_grid().map_err(|e| e.to_string())
}

// ─── Scheduler Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_scheduler_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conditions = state.scheduler.check_conditions();
    Ok(serde_json::json!({
        "isReady": conditions.all_met(),
        "conditions": {
            "userIsIdle": conditions.user_is_idle,
            "noFullscreen": conditions.no_fullscreen,
            "enoughTimeSinceLast": conditions.enough_time_since_last,
            "withinWorkHours": conditions.within_work_hours,
            "notPaused": conditions.not_paused,
            "hasDueExercises": conditions.has_due_exercises,
            "underDailyLimit": conditions.under_daily_limit,
        },
        "blockedReason": conditions.reason_blocked(),
    }))
}

#[tauri::command]
pub async fn set_scheduler_paused(
    paused: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.scheduler.set_paused(paused);
    Ok(())
}

// ─── Seed Data Command ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn seed_sample_words(state: State<'_, AppState>) -> Result<i32, String> {
    let words = sample_words();
    let mut count = 0;
    for word in words {
        if state.db.insert_word(&word).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

fn sample_words() -> Vec<Word> {
    let now = chrono::Utc::now();
    vec![
        Word {
            id: 0, term: "ephemeral".to_string(),
            definition: "Lasting for a very short time; transitory".to_string(),
            definition_pl: Some("Krótkotrwały, nietrwały, przemijający — istniejący tylko przez chwilę".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/ɪˈfem.ər.əl/".to_string()),
            examples: vec!["The ephemeral beauty of cherry blossoms makes them all the more precious.".to_string()],
            synonyms: vec!["transient".to_string(), "fleeting".to_string(), "momentary".to_string()],
            antonyms: vec!["permanent".to_string(), "enduring".to_string()],
            tags: vec!["common".to_string()], difficulty: 3, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "ubiquitous".to_string(),
            definition: "Present, appearing, or found everywhere".to_string(),
            definition_pl: Some("Wszechobecny, spotykany wszędzie — coś, co jest w każdym miejscu jednocześnie".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/juːˈbɪk.wɪ.təs/".to_string()),
            examples: vec!["Smartphones have become ubiquitous in modern society.".to_string()],
            synonyms: vec!["omnipresent".to_string(), "pervasive".to_string()],
            antonyms: vec!["rare".to_string(), "scarce".to_string()],
            tags: vec!["common".to_string()], difficulty: 3, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "serendipity".to_string(),
            definition: "The occurrence of fortunate events by accident or chance".to_string(),
            definition_pl: Some("Szczęśliwy przypadek — odkrycie czegoś wartościowego przez zrządzenie losu, bez szukania".to_string()),
            part_of_speech: "noun".to_string(),
            phonetic: Some("/ˌser.ənˈdɪp.ɪ.ti/".to_string()),
            examples: vec!["It was pure serendipity that they met at the coffee shop that day.".to_string()],
            synonyms: vec!["luck".to_string(), "fortune".to_string(), "chance".to_string()],
            antonyms: vec!["misfortune".to_string()],
            tags: vec!["popular".to_string()], difficulty: 2, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "perspicacious".to_string(),
            definition: "Having a ready insight into things; shrewd".to_string(),
            definition_pl: Some("Przenikliwy, bystrzy — ktoś, kto szybko rozumie i trafnie ocenia sytuacje".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/ˌpɜːr.spɪˈkeɪ.ʃəs/".to_string()),
            examples: vec!["The perspicacious investor saw the company's potential before anyone else.".to_string()],
            synonyms: vec!["astute".to_string(), "shrewd".to_string(), "perceptive".to_string()],
            antonyms: vec!["obtuse".to_string(), "dim-witted".to_string()],
            tags: vec!["advanced".to_string()], difficulty: 5, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "mellifluous".to_string(),
            definition: "Sweet or musical; pleasant to hear".to_string(),
            definition_pl: Some("Melodyjny, słodki w brzmieniu — dźwięk lub głos, który przyjemnie brzmi dla uszu".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/məˈlɪf.lu.əs/".to_string()),
            examples: vec!["Her mellifluous voice filled the concert hall with warmth.".to_string()],
            synonyms: vec!["dulcet".to_string(), "harmonious".to_string(), "melodious".to_string()],
            antonyms: vec!["harsh".to_string(), "discordant".to_string()],
            tags: vec!["literary".to_string()], difficulty: 4, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "pragmatic".to_string(),
            definition: "Dealing with things sensibly and realistically".to_string(),
            definition_pl: Some("Pragmatyczny, praktyczny — skupiony na tym, co działa w rzeczywistości, bez niepotrzebnego idealizmu".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/præɡˈmæt.ɪk/".to_string()),
            examples: vec!["She took a pragmatic approach to solving the budget crisis.".to_string()],
            synonyms: vec!["practical".to_string(), "sensible".to_string(), "realistic".to_string()],
            antonyms: vec!["idealistic".to_string(), "impractical".to_string()],
            tags: vec!["common".to_string()], difficulty: 2, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "inimical".to_string(),
            definition: "Tending to obstruct or harm; hostile or unfriendly".to_string(),
            definition_pl: Some("Wrogi, szkodliwy — coś lub ktoś, kto działa przeciwko czemuś lub komuś, utrudniając lub niszcząc".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/ɪˈnɪm.ɪ.kəl/".to_string()),
            examples: vec!["Such policies are inimical to economic growth.".to_string()],
            synonyms: vec!["hostile".to_string(), "antagonistic".to_string(), "adverse".to_string()],
            antonyms: vec!["friendly".to_string(), "beneficial".to_string()],
            tags: vec!["formal".to_string()], difficulty: 4, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
        Word {
            id: 0, term: "loquacious".to_string(),
            definition: "Tending to talk a great deal; talkative".to_string(),
            definition_pl: Some("Gadatliwy, wielomówny — osoba, która dużo i chętnie mówi, często za dużo".to_string()),
            part_of_speech: "adjective".to_string(),
            phonetic: Some("/ləʊˈkweɪ.ʃəs/".to_string()),
            examples: vec!["The loquacious professor often ran over time with his lectures.".to_string()],
            synonyms: vec!["talkative".to_string(), "garrulous".to_string(), "voluble".to_string()],
            antonyms: vec!["taciturn".to_string(), "reticent".to_string()],
            tags: vec!["character".to_string()], difficulty: 3, created_at: now, is_active: true, sentence_pl: None, sentence_en: None,
        },
    ]
}

// ─── Settings Commands ────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub exercises_per_day: u32,
    pub idle_threshold_secs: u32,
    pub min_gap_minutes: u32,
    pub autostart: bool,
    pub show_session_word: bool,
    pub sound_effects: bool,
    pub work_hours_only: bool,
    pub work_hours_start: String,
    pub work_hours_end: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            exercises_per_day: 50,
            idle_threshold_secs: 5,
            min_gap_minutes: 30,
            autostart: true,
            show_session_word: true,
            sound_effects: false,
            work_hours_only: true,
            work_hours_start: "08:00".to_string(),
            work_hours_end: "22:00".to_string(),
        }
    }
}

fn settings_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("settings.json")
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let path = settings_path(&state.data_dir);
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Persist to disk
    let path = settings_path(&state.data_dir);
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;

    // Apply to running scheduler immediately — no restart required
    use crate::learning::scheduler::SchedulerConfig;
    let new_config = SchedulerConfig {
        idle_threshold_secs: settings.idle_threshold_secs as u64,
        min_popup_gap_secs:  (settings.min_gap_minutes as u64) * 60,
        poll_interval_secs:  10,
        max_daily_exercises: settings.exercises_per_day as i32,
        work_hours_start:    crate::parse_hhmm_to_mins(&settings.work_hours_start, 8 * 60),
        work_hours_end:      crate::parse_hhmm_to_mins(&settings.work_hours_end,  22 * 60),
    };
    state.scheduler.update_config(new_config);
    Ok(())
}

// ─── Popup Window Control ─────────────────────────────────────────────────────

/// Called by popup window on mount — returns the queued exercise (no race condition)
#[tauri::command]
pub async fn get_popup_exercise(state: State<'_, AppState>) -> Result<Option<Exercise>, String> {
    let word_id = {
        let pending = state.pending_word_id.lock().map_err(|e| e.to_string())?;
        *pending
    };
    match word_id {
        Some(id) => state.engine.build_exercise(id).map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn hide_popup(window: tauri::Window) -> Result<(), String> {
    log::info!("[cmd] Closing popup window");
    window.close().map_err(|e| e.to_string())
}

/// Called from Dashboard "Ćwicz teraz" button — picks next due word and shows popup
#[tauri::command]
pub async fn trigger_popup(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    match state.db.get_session_word().map_err(|e| e.to_string())? {
        Some((word, _)) => {
            crate::show_popup(&app, word.id);
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Returns the word currently queued in the popup (for Dashboard "now practicing" widget)
#[tauri::command]
pub async fn get_current_word(state: State<'_, AppState>) -> Result<Option<Word>, String> {
    let word_id = {
        let pending = state.pending_word_id.lock().map_err(|e| e.to_string())?;
        *pending
    };
    match word_id {
        Some(id) => state.db.get_word_by_id(id).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

// ─── Task Notification Commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn task_notification_done(window: tauri::Window) -> Result<(), String> {
// pub async fn task_notification_done(app: tauri::AppHandle, window: tauri::Window) -> Result<(), String> {
    // Pobieramy ID słowa przed zamknięciem, jeśli potrzebne do logiki...
    window.close().map_err(|e| e.to_string())
}
/// User clicked "Później" or toast auto-closed → reset gap timer from NOW.
/// Does NOT count toward daily limit (user didn't actually do an exercise).
/// Next notification will appear after min_gap_minutes from this moment.
#[tauri::command]
pub async fn task_notification_later(
    word_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("task_notification_later: word_id={} — gap reset from now", word_id);
    state.scheduler.record_popup_dismissed(false);
    Ok(())
}

/// User clicked "Dobrze" — they already know the word.
/// Records a positive SM2 review (quality=5) without opening the exercise popup.
/// Counts toward the daily limit and resets the scheduler gap.
#[tauri::command]
pub async fn task_notification_known(
    word_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("task_notification_known: word_id={} — recording correct answer", word_id);
    let session_id = state.scheduler.session_id();
    state.engine
        .process_answer(
            word_id,
            true,                             // was_correct
            0,                                // response_time_ms (instant — no popup shown)
            None,                             // user_answer
            crate::db::ExerciseType::Introduction,
            &session_id,
        )
        .map_err(|e| e.to_string())?;
    state.scheduler.record_popup_dismissed(true);
    Ok(())
}

/// The answer a user gave on a flashcard.
/// Four-level SRS grade — maps directly to SM-2 quality (0–5).
///
/// | Button    | Quality | Meaning                              | SM-2 effect               |
/// |-----------|---------|--------------------------------------|---------------------------|
/// | Again     |    1    | Completely forgot / wrong            | Reset interval to ~10 min |
/// | Hard      |    3    | Recalled but with real effort        | Short interval, EF drops  |
/// | Good      |    4    | Recalled correctly                   | Normal interval            |
/// | Easy      |    5    | Instant recall, no hesitation        | Longer interval, EF rises |
#[derive(Debug, serde::Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SrsGrade { Again, Hard, Good, Easy }

impl SrsGrade {
    pub fn to_quality(self) -> i32 {
        match self {
            Self::Again => 1,
            Self::Hard  => 3,
            Self::Good  => 4,
            Self::Easy  => 5,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Again => "again",
            Self::Hard  => "hard",
            Self::Good  => "good",
            Self::Easy  => "easy",
        }
    }
    pub fn was_correct(self) -> bool {
        self != Self::Again
    }
}

/// Full SRS result payload — current-card outcome + next card data in one response.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SrsResult {
    // ── This card's outcome ──────────────────────────────────────────────────
    pub word_id:           i64,
    pub grade:             String,     // "again" | "hard" | "good" | "easy"
    pub new_mastery:       String,     // "new" | "learning" | "reviewing" | "mastered"
    pub new_interval_days: f64,
    pub new_easiness:      f64,
    pub streak:            i32,
    /// Human-readable next review time, e.g. "za 6 dni" or "za 10 minut"
    pub next_review_label: String,
    pub next_review_at:    String,
    // ── Next card (None = session complete / DB empty) ───────────────────────
    pub next_word_id:         Option<i64>,
    pub next_term_pl:         Option<String>,
    pub next_term_en:         Option<String>,
    pub next_part_of_speech:  Option<String>,
    pub next_phonetic:        Option<String>,
    pub next_sentence_pl:     Option<String>,
    pub next_sentence_en:     Option<String>,
}

/// Format a fractional-day interval into a Polish human-readable string.
fn format_interval_pl(days: f64) -> String {
    let minutes = (days * 24.0 * 60.0).round() as i64;
    match minutes {
        m if m < 1    => "teraz".into(),
        m if m < 60   => format!("za {} min", m),
        m if m < 1440 => format!("za {} h",   m / 60),
        d             => {
            let d = d / 1440;
            if d == 1 { "jutro".into() } else { format!("za {} dni", d) }
        }
    }
}

/// Primary SRS command called by the Flashcard after user grades a card.
///
/// Flow:
///   1. Map SrsGrade → SM-2 quality (1 / 3 / 4 / 5)
///   2. engine.process_answer() — updates word_progress + inserts exercise_history row
///   3. get_next_flashcard_word(exclude = word_id) — weighted priority SQL
///   4. Return SrsResult (outcome + next card) in a single response
#[tauri::command]
pub async fn srs_answer(
    word_id: i64,
    grade:   SrsGrade,
    state:   State<'_, AppState>,
) -> Result<SrsResult, String> {
    let session_id = state.scheduler.session_id();

    let result = state.engine
        .process_answer(
            word_id,
            grade.was_correct(),
            0,    // response_time_ms irrelevant for flashcards — quality drives SM-2
            None,
            crate::db::ExerciseType::Introduction,
            &session_id,
        )
        .map_err(|e| e.to_string())?;

    let next_review_label = format_interval_pl(result.new_interval_days);

    log::info!(
        "srs_answer: word='{}' grade={} quality={} → mastery={} interval={:.1}d ({}) ef={:.2} streak={}",
        result.word.term,
        grade.as_str(),
        grade.to_quality(),
        result.mastery_level,
        result.new_interval_days,
        next_review_label,
        result.new_ef,
        result.streak,
    );

    // ── Next card ─────────────────────────────────────────────────────────────
    let next = state.db
        .get_next_flashcard_word(Some(word_id))
        .map_err(|e| e.to_string())?;

    let (next_word_id, next_term_pl, next_term_en, next_part_of_speech,
         next_phonetic, next_sentence_pl, next_sentence_en) = match next {
        Some((w, _)) => {
            let term_pl = w.definition_pl.clone()
                .unwrap_or_else(|| w.definition.chars().take(60).collect());
            (Some(w.id), Some(term_pl), Some(w.term.clone()), Some(w.part_of_speech.clone()),
             w.phonetic.clone(), w.sentence_pl.clone(), w.sentence_en.clone())
        }
        None => (None, None, None, None, None, None, None),
    };

    // Bug fix: record this answer toward the daily limit.
    // Previously srs_answer never called record_popup_dismissed(), so
    // exercises_today stayed at 0 all day and the daily cap had no effect.
    state.scheduler.record_popup_dismissed(true);

    Ok(SrsResult {
        word_id,
        grade: grade.as_str().to_string(),
        new_mastery:       result.mastery_level,
        new_interval_days: result.new_interval_days,
        new_easiness:      result.new_ef,
        streak:            result.streak,
        next_review_label,
        next_review_at:    result.next_review_at,
        next_word_id,
        next_term_pl,
        next_term_en,
        next_part_of_speech,
        next_phonetic,
        next_sentence_pl,
        next_sentence_en,
    })
}

// Keep flashcard_answer for backwards compat — delegates to srs_answer logic
#[allow(dead_code)]
pub enum FlashcardDecision { Known, Practice }

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashcardResult {
    pub word_id: i64, pub decision: String, pub new_mastery: String,
    pub new_interval_days: f64, pub streak: i32,
    pub next_word_id: Option<i64>, pub next_term_pl: Option<String>,
    pub next_term_en: Option<String>, pub next_part_of_speech: Option<String>,
}

#[tauri::command]
pub async fn flashcard_answer(
    word_id: i64,
    decision: String,   // "known" | "practice"
    state: State<'_, AppState>,
) -> Result<FlashcardResult, String> {
    let grade = if decision == "known" { SrsGrade::Good } else { SrsGrade::Again };
    let srs = srs_answer(word_id, grade, state).await?;
    Ok(FlashcardResult {
        word_id: srs.word_id, decision, new_mastery: srs.new_mastery,
        new_interval_days: srs.new_interval_days, streak: srs.streak,
        next_word_id: srs.next_word_id, next_term_pl: srs.next_term_pl,
        next_term_en: srs.next_term_en, next_part_of_speech: srs.next_part_of_speech,
    })
}


// ─── JSON Import ──────────────────────────────────────────────────────────────

/// Minimal structure expected in an imported JSON file.
/// All fields except `term` and `definition` are optional — missing values
/// fall back to sensible defaults so partial data still imports cleanly.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedWord {
    pub term:            String,
    pub definition:      String,
    #[serde(default)] pub definition_pl:   Option<String>,
    #[serde(default)] pub part_of_speech:  Option<String>,
    #[serde(default)] pub phonetic:        Option<String>,
    #[serde(default)] pub examples:        Vec<String>,
    #[serde(default)] pub synonyms:        Vec<String>,
    #[serde(default)] pub antonyms:        Vec<String>,
    #[serde(default)] pub tags:            Vec<String>,
    #[serde(default)] pub difficulty:      Option<i32>,
    #[serde(default, rename = "zdaniePL")] pub sentence_pl: Option<String>,
    #[serde(default, rename = "zdanieEN")] pub sentence_en: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added:    usize,
    pub skipped:  usize,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn import_words_from_json(
    json: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    // ── Parse JSON ────────────────────────────────────────────────────────────
    let items: Vec<ImportedWord> = serde_json::from_str(&json)
        .map_err(|e| format!("Niepoprawny format JSON: {}", e))?;

    if items.is_empty() {
        return Ok(ImportResult { added: 0, skipped: 0, warnings: vec!["Plik JSON jest pusty.".into()] });
    }

    let mut added    = 0usize;
    let mut skipped  = 0usize;
    let mut warnings = Vec::new();

    for (i, item) in items.into_iter().enumerate() {
        let label = format!("#{} \"{}\"", i + 1, item.term);

        // ── Validate required fields ──────────────────────────────────────────
        let term = item.term.trim().to_string();
        let definition = item.definition.trim().to_string();

        if term.is_empty() {
            warnings.push(format!("{}: pominięto — brak pola `term`", label));
            skipped += 1;
            continue;
        }
        if definition.is_empty() {
            warnings.push(format!("{}: pominięto — brak pola `definition`", label));
            skipped += 1;
            continue;
        }

        let difficulty = item.difficulty
            .map(|d| d.clamp(1, 5))
            .unwrap_or(2);

        let word = crate::db::Word {
            id: 0,
            term,
            definition,
            definition_pl:  item.definition_pl,
            part_of_speech: item.part_of_speech.unwrap_or_else(|| "noun".to_string()),
            phonetic:       item.phonetic,
            examples:       item.examples,
            synonyms:       item.synonyms,
            antonyms:       item.antonyms,
            tags:           item.tags,
            difficulty,
            created_at:     chrono::Utc::now(),
            is_active:      true,
            sentence_pl:    item.sentence_pl,
            sentence_en:    item.sentence_en,
        };

        // ── Insert; detect UNIQUE constraint violation = duplicate ────────────
        match state.db.insert_word(&word) {
            Ok(_)  => { added += 1; }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("UNIQUE") || msg.contains("unique") {
                    warnings.push(format!("{}: pominięto — duplikat (słowo już istnieje)", label));
                    skipped += 1;
                } else {
                    warnings.push(format!("{}: błąd zapisu — {}", label, msg));
                    skipped += 1;
                }
            }
        }
    }

    log::info!("import_words_from_json: added={} skipped={}", added, skipped);
    Ok(ImportResult { added, skipped, warnings })
}
#[tauri::command]
pub async fn initialize_autostart(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let flag_path = data_dir.join(".autostart_initialized");

    // Wykonaj tylko, jeśli aplikacja nie była jeszcze inicjalizowana
    if !flag_path.exists() {
        log::info!("Pierwsze uruchomienie: Konfiguracja autostartu...");
        let _ = app.autolaunch().enable();
        
        // Tworzymy pusty plik jako znacznik ukończenia konfiguracji
        std::fs::write(flag_path, "1").map_err(|e| e.to_string())?;
    }
    
    Ok(())
}