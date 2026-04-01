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

// ─── Constants ────────────────────────────────────────────────────────────────

pub const GLOBAL_CATEGORIES: &[&str] = &[
    "IT", "biznes", "czasowniki frazowe", "osobowość", "uroda i pielęgnacja",
    "wygląd zewnętrzny", "ubrania", "rodzina i związki", "sport i czas wolny",
    "jedzenie i gotowanie", "samopoczucie", "zdrowie i choroby", "ciało i organy",
    "polityka i społeczeństwo", "historia", "kultura i sztuka", "wojny i katastrofy",
    "samochód", "podróżowanie", "praca", "finanse i biznes", "sprzedaż i marketing",
    "prawo i przestępczość", "technologia", "nauka i badania", "edukacja",
    "wyrażenia przyimkowe", "bez kategorii"
];

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
    let active_cat = state.scheduler.active_category();
    match state.engine.start_session(active_cat) {
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
    pub category:       Option<String>,
    pub created_at:     String,
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
            category:       word.category.clone(),
            created_at:     word.created_at.to_rfc3339(),
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
    category: Option<String>,
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
        category,
    };
    state.db.insert_word(&word).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_word_category(
    id: i64,
    category: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.update_word_category(id, category).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReclassifyPayload {
    pub words: Vec<Word>,
    pub categories: Vec<String>,
}

#[tauri::command]
pub async fn reclassify_words(state: State<'_, AppState>) -> Result<ReclassifyPayload, String> {
    let all_words = state.db.get_all_words().map_err(|e| e.to_string())?;
    
    // Filtrujemy tylko te, które nie mają przypisanej kategorii
    let words_to_reclassify: Vec<Word> = all_words
        .into_iter()
        .filter(|w| w.category.is_none() || w.category.as_deref() == Some("bez kategorii"))
        .collect();

    let categories: Vec<String> = GLOBAL_CATEGORIES.iter().map(|s| s.to_string()).collect();

    Ok(ReclassifyPayload {
        words: words_to_reclassify,
        categories,
    })
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

#[tauri::command]
pub async fn delete_words_by_batch_date(date: String, state: State<'_, AppState>) -> Result<usize, String> {
    let deleted = state.db.delete_words_by_date(&date).map_err(|e| e.to_string())?;
    log::info!("delete_words_by_batch_date: deleted {} words from date {}", deleted, date);
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

#[tauri::command]
pub async fn set_active_category(
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.scheduler.set_active_category(category.clone());
    
    // Persist choice to settings.json
    let mut settings = get_settings(state.clone()).await.unwrap_or_else(|_| AppSettings::default());
    settings.active_category = category;
    save_settings(settings, state).await?;
    
    Ok(())
}

// #[tauri::command]
// pub async fn repair_json_data(state: State<'_, AppState>) -> Result<String, String> {
//     // Zakładamy, że skrypt jest w głównym katalogu projektu.
//     // Jeśli jest inaczej, ścieżkę trzeba będzie dostosować.
//     let script_path = "../repair_fiszki.py"; 
    
//     // Sprawdzamy, czy skrypt istnieje
//     if !std::path::Path::new(script_path).exists() {
//         return Err(format!("Skrypt {} nie został znaleziony. Upewnij się, że jest w głównym katalogu projektu.", script_path));
//     }

//     // Uruchamiamy skrypt Pythona
//     // Ważne: Używamy `powershell.exe -NoProfile -Command` dla spójności z innymi komendami.
//     // Upewnij się, że Python jest dostępny w ścieżce systemowej lub podaj pełną ścieżkę do interpretera.
//     // Dodajemy PAGER=cat, żeby komenda nie czekała na interakcję, jeśli coś by się wyświetliło
//     let command = format!("$env:PAGER='cat'; python \"{}\"", script_path);

//     let output = run_shell_command(
//         command,
//         Some("Uruchamiam skrypt naprawy danych JSON...".to_string()),
//         None, // Domyślnie bieżący katalog
//         false,
//     ).await;

//     // Sprawdzamy wyjście i kod zakończenia
//     if output.exit_code.is_some() && output.exit_code != Some(0) {
//         return Err(format!("Skrypt naprawy zakończył się błędem (kod {}):\n{}", output.exit_code.unwrap(), output.output));
//     } else if !output.error.is_none() {
//         return Err(format!("Błąd wykonania skryptu: {}\n{}", output.error.unwrap(), output.output));
//     } else if output.output.is_empty() || output.output.contains("Nie znaleziono pliku") {
//         // Bardziej specyficzny błąd dla nieznalezienia pliku JSON
//         return Err(format!("Błąd wykonania skryptu: Nie znaleziono pliku {}. Sprawdź ścieżkę i nazwę pliku.", FILE_PATH));
//     } else {
//         log::info!("Skrypt naprawy zakończony pomyślnie. Wynik: {}", output.output);
//         // Można by tu bardziej szczegółowo analizować output, ale na razie zakładamy sukces jeśli brak błędów
//         return Ok(format!("Naprawa zakończona. Wynik:\n{}", output.output));
//     Ok(())
//     }

    // #[tauri::command]
    // pub async fn repair_json_data(state: State<'_, AppState>) -> Result<String, String> {
    // // Zakładamy, że skrypt jest w głównym katalogu projektu.
    // // Jeśli jest inaczej, ścieżkę trzeba będzie dostosować.
    // let script_path = "../repair_fiszki.py"; 

    // // Sprawdzamy, czy skrypt istnieje
    // if !std::path::Path::new(script_path).exists() {
    //     return Err(format!("Skrypt {} nie został znaleziony. Upewnij się, że jest w głównym katalogu projektu.", script_path));
    // }

    // // Uruchamiamy skrypt Pythona
    // // Ważne: Używamy `powershell.exe -NoProfile -Command` dla spójności z innymi komendami.
    // // Upewnij się, że Python jest dostępny w ścieżce systemowej lub podaj pełną ścieżkę do interpretera.
    // // Dodajemy PAGER=cat, żeby komenda nie czekała na interakcję, jeśli coś by się wyświetliło
    // let command = format!("$env:PAGER='cat'; python \"{}\"", script_path);

    // let output = run_shell_command(
    //     command,
    //     Some("Uruchamiam skrypt naprawy danych JSON...".to_string()),
    //     None, // Domyślnie bieżący katalog
    //     false,
    // ).await;

    // // Sprawdzamy wyjście i kod zakończenia
    // if output.exit_code.is_some() && output.exit_code != Some(0) {
    //     return Err(format!("Skrypt naprawy zakończył się błędem (kod {}):\n{}", output.exit_code.unwrap(), output.output));
    // } else if !output.error.is_none() {
    //     return Err(format!("Błąd wykonania skryptu: {}\n{}", output.error.unwrap(), output.output));
    // } else if output.output.is_empty() || output.output.contains("Nie znaleziono pliku") {
    //     // Bardziej specyficzny błąd dla nieznalezienia pliku JSON
    //     return Err(format!("Błąd wykonania skryptu: Nie znaleziono pliku {}. Sprawdź ścieżkę i nazwę pliku.", FILE_PATH));
    // } else {
    //     log::info!("Skrypt naprawy zakończony pomyślnie. Wynik: {}", output.output);
    //     // Można by tu bardziej szczegółowo analizować output, ale na razie zakładamy sukces jeśli brak błędów
    //     return Ok(format!("Naprawa zakończona. Wynik:\n{}", output.output));
    // }
    // }

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
    pub active_category: Option<String>,
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
            active_category: None,
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
    let active_cat = state.scheduler.active_category();
    match state.db.get_session_word(active_cat).map_err(|e| e.to_string())? {
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
    let active_cat = state.scheduler.active_category();
    let next = state.db
        .get_next_flashcard_word(Some(word_id), active_cat)
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
    #[serde(default)] pub category:       Option<String>,
    #[serde(default)] pub created_at:     Option<String>,
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

    let batch_date = chrono::Utc::now();

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
            created_at:     batch_date,
            is_active:      true,
            sentence_pl:    item.sentence_pl,
            sentence_en:    item.sentence_en,
            category:       item.category,
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

        // ─── AI Mentor Commands ───────────────────────────────────────────────────────

        #[tauri::command]
        pub async fn get_struggling_words(
        limit: i32,
        category_filter: Option<String>,
        state: State<'_, AppState>,
        ) -> Result<Vec<Word>, String> {
        state.db.get_struggling_words(limit, category_filter).map_err(|e| e.to_string())
        }

        #[tauri::command]
        pub async fn get_next_review_word(
            category_filter: Option<String>,
            state: State<'_, AppState>,
        ) -> Result<Option<Word>, String> {
            match state.db.get_next_flashcard_word(None, category_filter).map_err(|e| e.to_string())? {
                Some((word, _)) => Ok(Some(word)),
                None => Ok(None),
            }
        }

        #[tauri::command]
        pub async fn get_mentor_tips(
        state: State<'_, AppState>,
        ) -> Result<serde_json::Value, String> {
        let path = state.data_dir.join("mentor-tips.json");
        if !path.exists() {
            return Ok(serde_json::json!({}));
        }
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(json)
        }

        #[tauri::command]
        pub async fn save_mentor_tips(
        tips: serde_json::Value,
        state: State<'_, AppState>,
        ) -> Result<(), String> {
        let path = state.data_dir.join("mentor-tips.json");
        let content = serde_json::to_string_pretty(&tips).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
        }
    