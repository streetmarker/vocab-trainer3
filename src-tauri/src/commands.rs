// src-tauri/src/commands.rs
//
// All Tauri commands exposed to the frontend via invoke().

use std::path::PathBuf;

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
    };
    state.db.insert_word(&word).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_word(word_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    state.db.delete_word(word_id).map_err(|e| e.to_string())
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
            tags: vec!["common".to_string()], difficulty: 3, created_at: now, is_active: true,
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
            tags: vec!["common".to_string()], difficulty: 3, created_at: now, is_active: true,
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
            tags: vec!["popular".to_string()], difficulty: 2, created_at: now, is_active: true,
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
            tags: vec!["advanced".to_string()], difficulty: 5, created_at: now, is_active: true,
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
            tags: vec!["literary".to_string()], difficulty: 4, created_at: now, is_active: true,
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
            tags: vec!["common".to_string()], difficulty: 2, created_at: now, is_active: true,
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
            tags: vec!["formal".to_string()], difficulty: 4, created_at: now, is_active: true,
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
            tags: vec!["character".to_string()], difficulty: 3, created_at: now, is_active: true,
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
    let path = settings_path(&state.data_dir);
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}
