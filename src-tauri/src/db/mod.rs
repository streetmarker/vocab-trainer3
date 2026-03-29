// src-tauri/src/db/mod.rs
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Domain Models ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub id: i64,
    pub term: String,
    pub definition: String,
    pub definition_pl: Option<String>,   // Polish translation / explanation
    pub part_of_speech: String,
    pub phonetic: Option<String>,
    pub examples: Vec<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub tags: Vec<String>,
    pub difficulty: i32,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub sentence_pl: Option<String>,     // Example sentence in Polish (bold term on flashcard front)
    pub sentence_en: Option<String>,     // Example sentence in English (bold term on flashcard back)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordProgress {
    pub id: i64,
    pub word_id: i64,
    pub easiness_factor: f64,
    pub interval_days: f64,
    pub repetitions: i32,
    pub iterations: i32,
    pub next_review_at: DateTime<Utc>,
    pub last_review_at: Option<DateTime<Utc>>,
    pub total_reviews: i32,
    pub correct_reviews: i32,
    pub streak: i32,
    pub introduced_at: Option<DateTime<Utc>>,
    pub session_reviews: i32,
    pub next_session_review_at: Option<DateTime<Utc>>,
    pub mastery_level: MasteryLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MasteryLevel {
    New, Learning, Reviewing, Mastered,
}

impl std::fmt::Display for MasteryLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New       => write!(f, "new"),
            Self::Learning  => write!(f, "learning"),
            Self::Reviewing => write!(f, "reviewing"),
            Self::Mastered  => write!(f, "mastered"),
        }
    }
}

impl MasteryLevel {
    pub fn from_str(s: &str) -> Self {
        match s {
            "learning"  => Self::Learning,
            "reviewing" => Self::Reviewing,
            "mastered"  => Self::Mastered,
            _           => Self::New,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::New       => "new",
            Self::Learning  => "learning",
            Self::Reviewing => "reviewing",
            Self::Mastered  => "mastered",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseHistory {
    pub id: i64,
    pub word_id: i64,
    pub exercise_type: ExerciseType,
    pub response_time_ms: i64,
    pub quality: i32,
    pub was_correct: bool,
    pub user_answer: Option<String>,
    pub session_id: String,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExerciseType {
    Introduction, MultipleChoice, FillInBlank, ContextualGuess,
    SpellingCheck, SynonymMatch, DefinitionRecall, TrueFalse,
}

impl ExerciseType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "introduction"     => Self::Introduction,
            "multiple_choice"  => Self::MultipleChoice,
            "fill_in_blank"    => Self::FillInBlank,
            "contextual_guess" => Self::ContextualGuess,
            "spelling_check"   => Self::SpellingCheck,
            "synonym_match"    => Self::SynonymMatch,
            "definition_recall"=> Self::DefinitionRecall,
            _                  => Self::TrueFalse,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Introduction   => "introduction",
            Self::MultipleChoice => "multiple_choice",
            Self::FillInBlank    => "fill_in_blank",
            Self::ContextualGuess=> "contextual_guess",
            Self::SpellingCheck  => "spelling_check",
            Self::SynonymMatch   => "synonym_match",
            Self::DefinitionRecall => "definition_recall",
            Self::TrueFalse      => "true_false",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStats {
    pub date: String,
    pub exercises_completed: i32,
    pub correct_answers: i32,
    pub words_reviewed: i32,
    pub words_mastered: i32,
    pub total_time_ms: i64,
    pub streak_days: i32,
}

// ─── Database Manager ─────────────────────────────────────────────────────────

pub struct Database {
    conn: parking_lot::Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=10000;
             PRAGMA foreign_keys=ON;",
        )?;
        let db = Self { conn: parking_lot::Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(SCHEMA_SQL)?;
        // Non-destructive migrations: ignore errors if column already exists
        let _ = conn.execute_batch("ALTER TABLE word ADD COLUMN definition_pl TEXT;");
        let _ = conn.execute_batch("ALTER TABLE word ADD COLUMN sentence_pl TEXT;");
        let _ = conn.execute_batch("ALTER TABLE word ADD COLUMN sentence_en TEXT;");
        let _ = conn.execute_batch("ALTER TABLE word_progress ADD COLUMN iterations INTEGER NOT NULL DEFAULT 0;");
        Ok(())
    }

    // ─── Word Queries ──────────────────────────────────────────────────────

    pub fn insert_word(&self, word: &Word) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO word (term, definition, definition_pl, part_of_speech, phonetic,
             examples, synonyms, antonyms, tags, difficulty, created_at, is_active,
             sentence_pl, sentence_en)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                word.term, word.definition, word.definition_pl,
                word.part_of_speech, word.phonetic,
                serde_json::to_string(&word.examples)?,
                serde_json::to_string(&word.synonyms)?,
                serde_json::to_string(&word.antonyms)?,
                serde_json::to_string(&word.tags)?,
                word.difficulty, word.created_at.to_rfc3339(), word.is_active,
                word.sentence_pl, word.sentence_en,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_all_words(&self) -> Result<Vec<Word>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, term, definition, definition_pl, part_of_speech, phonetic,
             examples, synonyms, antonyms, tags, difficulty, created_at, is_active,
             sentence_pl, sentence_en
             FROM word WHERE is_active = 1 ORDER BY term",
        )?;
        let words = stmt.query_map([], row_to_word)?.collect::<Result<Vec<_>, _>>()?;
        Ok(words)
    }

    pub fn get_word_by_id(&self, id: i64) -> Result<Option<Word>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, term, definition, definition_pl, part_of_speech, phonetic,
             examples, synonyms, antonyms, tags, difficulty, created_at, is_active,
             sentence_pl, sentence_en
             FROM word WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], row_to_word)?;
        Ok(rows.next().transpose()?)
    }

    pub fn get_words_for_distractor(&self, exclude_id: i64, limit: usize) -> Result<Vec<Word>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, term, definition, definition_pl, part_of_speech, phonetic,
             examples, synonyms, antonyms, tags, difficulty, created_at, is_active,
             sentence_pl, sentence_en
             FROM word WHERE id != ?1 AND is_active = 1
             ORDER BY RANDOM() LIMIT ?2",
        )?;
        let words = stmt
            .query_map(params![exclude_id, limit as i64], row_to_word)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(words)
    }

    pub fn delete_word(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM word WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Permanently deletes ALL words and their associated progress / history rows.
    /// CASCADE foreign keys handle word_progress and exercise_history automatically.
    pub fn clear_all_words(&self) -> Result<usize> {
        let conn = self.conn.lock();
        let deleted = conn.execute("DELETE FROM word", [])?;
        Ok(deleted)
    }

    pub fn get_struggling_words(&self, limit: i32) -> Result<Vec<Word>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT w.id, w.term, w.definition, w.definition_pl, w.part_of_speech, w.phonetic,
             w.examples, w.synonyms, w.antonyms, w.tags, w.difficulty, w.created_at, w.is_active,
             w.sentence_pl, w.sentence_en
             FROM word w
             JOIN exercise_history h ON w.id = h.word_id
             WHERE w.is_active = 1
             GROUP BY w.id
             HAVING COUNT(CASE WHEN h.was_correct = 0 THEN 1 END) >= 1
                OR AVG(h.response_time_ms) > 3000
             ORDER BY COUNT(CASE WHEN h.was_correct = 0 THEN 1 END) DESC, AVG(h.response_time_ms) DESC
             LIMIT ?1",
        )?;
        let words = stmt.query_map([limit], row_to_word)?.collect::<Result<Vec<_>, _>>()?;
        Ok(words)
    }

    // ─── Progress Queries ──────────────────────────────────────────────────

    pub fn get_or_create_progress(&self, word_id: i64) -> Result<WordProgress> {
        let conn = self.conn.lock();
        let existing: Option<WordProgress> = {
            let mut stmt = conn.prepare(
                "SELECT id, word_id, easiness_factor, interval_days, repetitions, iterations,
                 next_review_at, last_review_at, total_reviews, correct_reviews,
                 streak, introduced_at, session_reviews, next_session_review_at, mastery_level
                 FROM word_progress WHERE word_id = ?1",
            )?;
            let result = stmt.query_map([word_id], row_to_progress)?.next().transpose()?;
            result
        };
        if let Some(p) = existing { return Ok(p); }

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO word_progress (word_id, easiness_factor, interval_days, repetitions, iterations,
             next_review_at, total_reviews, correct_reviews, streak, session_reviews, mastery_level)
             VALUES (?1, 2.5, 0.0, 0, 0, ?2, 0, 0, 0, 0, 'new')",
            params![word_id, now],
        )?;
        let id = conn.last_insert_rowid();
        Ok(WordProgress {
            id, word_id, easiness_factor: 2.5, interval_days: 0.0,
            repetitions: 0, iterations: 0,
            next_review_at: Utc::now(), last_review_at: None, total_reviews: 0,
            correct_reviews: 0, streak: 0, introduced_at: None, session_reviews: 0,
            next_session_review_at: None, mastery_level: MasteryLevel::New,
        })
    }

    pub fn update_progress(&self, progress: &WordProgress) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE word_progress SET
             easiness_factor=?1, interval_days=?2, repetitions=?3, iterations=?4,
             next_review_at=?5, last_review_at=?6, total_reviews=?7,
             correct_reviews=?8, streak=?9, introduced_at=?10,
             session_reviews=?11, next_session_review_at=?12, mastery_level=?13
             WHERE id=?14",
            params![
                progress.easiness_factor, progress.interval_days, progress.repetitions,
                progress.iterations,
                progress.next_review_at.to_rfc3339(),
                progress.last_review_at.map(|d| d.to_rfc3339()),
                progress.total_reviews, progress.correct_reviews, progress.streak,
                progress.introduced_at.map(|d| d.to_rfc3339()),
                progress.session_reviews,
                progress.next_session_review_at.map(|d| d.to_rfc3339()),
                progress.mastery_level.as_str(), progress.id,
            ],
        )?;
        Ok(())
    }

    pub fn get_due_words(&self) -> Result<Vec<(Word, WordProgress)>> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT w.id, w.term, w.definition, w.definition_pl, w.part_of_speech, w.phonetic,
             w.examples, w.synonyms, w.antonyms, w.tags, w.difficulty, w.created_at, w.is_active,
             w.sentence_pl, w.sentence_en,
             p.id, p.word_id, p.easiness_factor, p.interval_days, p.repetitions, p.iterations,
             p.next_review_at, p.last_review_at, p.total_reviews, p.correct_reviews,
             p.streak, p.introduced_at, p.session_reviews, p.next_session_review_at, p.mastery_level
             FROM word w
             JOIN word_progress p ON w.id = p.word_id
             WHERE w.is_active = 1 AND p.next_review_at <= ?1
             ORDER BY p.next_review_at ASC",
        )?;
        let pairs = stmt.query_map([now], |row| {
            let word = Word {
                id: row.get(0)?, term: row.get(1)?, definition: row.get(2)?,
                definition_pl: row.get(3)?, part_of_speech: row.get(4)?,
                phonetic: row.get(5)?,
                examples: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                synonyms: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                antonyms: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                tags:     serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                difficulty: row.get(10)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?).unwrap().with_timezone(&Utc),
                is_active: row.get(12)?,
                sentence_pl: row.get(13)?,
                sentence_en: row.get(14)?,
            };
            let progress = WordProgress {
                id: row.get(15)?, word_id: row.get(16)?,
                easiness_factor: row.get(17)?, interval_days: row.get(18)?,
                repetitions: row.get(19)?, iterations: row.get(20)?,
                next_review_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(21)?).unwrap().with_timezone(&Utc),
                last_review_at: row.get::<_, Option<String>>(22)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
                total_reviews: row.get(23)?, correct_reviews: row.get(24)?, streak: row.get(25)?,
                introduced_at: row.get::<_, Option<String>>(26)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
                session_reviews: row.get(27)?,
                next_session_review_at: row.get::<_, Option<String>>(28)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
                mastery_level: MasteryLevel::from_str(&row.get::<_, String>(29).unwrap_or_default()),
            };
            Ok((word, progress))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(pairs)
    }

    // ─── Exercise History ──────────────────────────────────────────────────

    pub fn record_exercise(&self, ex: &ExerciseHistory) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO exercise_history
             (word_id, exercise_type, response_time_ms, quality, was_correct,
             user_answer, session_id, completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                ex.word_id, ex.exercise_type.as_str(), ex.response_time_ms,
                ex.quality, ex.was_correct, ex.user_answer, ex.session_id,
                ex.completed_at.to_rfc3339(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_daily_stats(&self, days: i32) -> Result<Vec<DailyStats>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT date(completed_at) as day,
               COUNT(*) as total,
               SUM(CASE WHEN was_correct THEN 1 ELSE 0 END) as correct,
               COUNT(DISTINCT word_id) as words,
               SUM(response_time_ms) as total_ms
             FROM exercise_history
             WHERE completed_at >= date('now', ?1)
             GROUP BY day ORDER BY day",
        )?;
        let stats = stmt.query_map([format!("-{} days", days)], |row| {
            Ok(DailyStats {
                date: row.get(0)?, exercises_completed: row.get(1)?,
                correct_answers: row.get(2)?, words_reviewed: row.get(3)?,
                words_mastered: 0, total_time_ms: row.get(4)?, streak_days: 0,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(stats)
    }

    pub fn get_overall_stats(&self) -> Result<serde_json::Value> {
        let conn = self.conn.lock();
        let total_words: i32 = conn.query_row("SELECT COUNT(*) FROM word WHERE is_active=1", [], |r| r.get(0))?;
        let mastered: i32 = conn.query_row("SELECT COUNT(*) FROM word_progress WHERE mastery_level='mastered'", [], |r| r.get(0))?;
        let total_exercises: i32 = conn.query_row("SELECT COUNT(*) FROM exercise_history", [], |r| r.get(0))?;
        let correct: i32 = conn.query_row("SELECT COUNT(*) FROM exercise_history WHERE was_correct=1", [], |r| r.get(0))?;
        let streak: i32 = self.calculate_streak(&conn)?;
        Ok(serde_json::json!({
            "totalWords": total_words, "masteredWords": mastered,
            "totalExercises": total_exercises, "correctAnswers": correct,
            "accuracyPercent": if total_exercises > 0 { correct * 100 / total_exercises } else { 0 },
            "currentStreak": streak,
        }))
    }

    fn calculate_streak(&self, conn: &Connection) -> Result<i32> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT date(completed_at) as day FROM exercise_history
             ORDER BY day DESC LIMIT 365",
        )?;
        let days: Vec<String> = stmt.query_map([], |r| r.get(0))?.filter_map(|r| r.ok()).collect();
        let mut streak = 0i32;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let mut expected = today.clone();
        for day in &days {
            if *day == expected {
                streak += 1;
                let d = chrono::NaiveDate::parse_from_str(&expected, "%Y-%m-%d").unwrap();
                expected = (d - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
            } else { break; }
        }
        Ok(streak)
    }

    pub fn get_session_word(&self) -> Result<Option<(Word, WordProgress)>> {
        self.get_next_flashcard_word(None)
    }

    /// Select the next word for a flashcard session using a weighted scoring algorithm.
    ///
    /// # Scoring formula (higher score = shown sooner)
    ///
    /// | Component                        | Value        | Rationale                          |
    /// |----------------------------------|--------------|------------------------------------|
    /// | mastery = new (no progress yet)  | +40          | New words introduced first         |
    /// | mastery = learning               | +30          | "Do ćwiczenia" words boosted       |
    /// | mastery = reviewing              | +10          | Active review                      |
    /// | mastery = mastered               |  +0          | Lowest priority                    |
    /// | SM-2 review overdue              | +25          | Long overdue gets strong boost     |
    /// | SM-2 review due today            | +15          | Due today gets moderate boost      |
    /// | repetitions = 0                  | +20          | Never practiced                    |
    /// | repetitions 1-2                  | +10          | Rarely practiced                   |
    /// | repetitions 3-5                  |  +5          | Some practice                      |
    /// | RANDOM() × 8                     | 0-8          | Prevent identical order every time |
    ///
    /// # Parameters
    /// - `exclude_id`: word_id just answered — excluded so the same card never
    ///   appears twice in a row (unless it's the only word in the DB)
    pub fn get_next_flashcard_word(&self, exclude_id: Option<i64>) -> Result<Option<(Word, WordProgress)>> {
        let conn = self.conn.lock();
        let now  = Utc::now().to_rfc3339();
        let excl = exclude_id.unwrap_or(-1); // -1 matches nothing (no word has id=-1)

        let word_id: rusqlite::Result<i64> = conn.query_row(
            "SELECT w.id
             FROM word w
             LEFT JOIN word_progress p ON w.id = p.word_id
             WHERE w.is_active = 1
               AND w.id != ?2
               AND (
                 p.next_review_at IS NULL 
                 OR p.next_review_at <= ?1
                 OR p.mastery_level = 'new'
               )
             ORDER BY 
               -- 1. Pilne powtórki (najpierw najbardziej opóźnione)
               CASE WHEN p.next_review_at <= ?1 THEN p.next_review_at ELSE '9999' END ASC,
               -- 2. Nowe słowa (zawsze w drugiej kolejności)
               CASE WHEN p.mastery_level = 'new' OR p.mastery_level IS NULL THEN 0 ELSE 1 END ASC,
               -- 3. Mała losowość dla słów o tym samym priorytecie
               RANDOM()
             LIMIT 1",
            rusqlite::params![now, excl],
            |r| r.get::<_, i64>(0),
        );

        match word_id {
            Ok(id) => {
                drop(conn);
                let word = self.get_word_by_id(id)?;
                if let Some(w) = word {
                    let progress = self.get_or_create_progress(w.id)?;
                    Ok(Some((w, progress)))
                } else {
                    Ok(None)
                }
            }
            // No rows = empty DB or all due words completed.
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

}

// ─── Row Mappers ──────────────────────────────────────────────────────────────

fn row_to_word(row: &rusqlite::Row) -> rusqlite::Result<Word> {
    Ok(Word {
        id: row.get(0)?, term: row.get(1)?, definition: row.get(2)?,
        definition_pl: row.get(3)?, part_of_speech: row.get(4)?,
        phonetic: row.get(5)?,
        examples: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
        synonyms: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
        antonyms: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
        tags:     serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
        difficulty: row.get(10)?,
        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?).unwrap().with_timezone(&Utc),
        is_active: row.get(12)?,
        sentence_pl: row.get(13)?,
        sentence_en: row.get(14)?,
    })
}

fn row_to_progress(row: &rusqlite::Row) -> rusqlite::Result<WordProgress> {
    Ok(WordProgress {
        id: row.get(0)?, word_id: row.get(1)?,
        easiness_factor: row.get(2)?, interval_days: row.get(3)?,
        repetitions: row.get(4)?, iterations: row.get(5)?,
        next_review_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap().with_timezone(&Utc),
        last_review_at: row.get::<_, Option<String>>(7)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
        total_reviews: row.get(8)?, correct_reviews: row.get(9)?, streak: row.get(10)?,
        introduced_at: row.get::<_, Option<String>>(11)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
        session_reviews: row.get(12)?,
        next_session_review_at: row.get::<_, Option<String>>(13)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)),
        mastery_level: MasteryLevel::from_str(&row.get::<_, String>(14).unwrap_or_default()),
    })
}

// ─── Schema SQL ───────────────────────────────────────────────────────────────

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS word (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    term            TEXT NOT NULL UNIQUE,
    definition      TEXT NOT NULL,
    definition_pl   TEXT,
    part_of_speech  TEXT NOT NULL DEFAULT 'noun',
    phonetic        TEXT,
    examples        TEXT NOT NULL DEFAULT '[]',
    synonyms        TEXT NOT NULL DEFAULT '[]',
    antonyms        TEXT NOT NULL DEFAULT '[]',
    tags            TEXT NOT NULL DEFAULT '[]',
    difficulty      INTEGER NOT NULL DEFAULT 2 CHECK(difficulty BETWEEN 1 AND 5),
    created_at      TEXT NOT NULL,
    is_active       INTEGER NOT NULL DEFAULT 1,
    sentence_pl     TEXT,
    sentence_en     TEXT
);

CREATE TABLE IF NOT EXISTS word_progress (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id                 INTEGER NOT NULL UNIQUE REFERENCES word(id) ON DELETE CASCADE,
    easiness_factor         REAL NOT NULL DEFAULT 2.5,
    interval_days           REAL NOT NULL DEFAULT 0.0,
    repetitions             INTEGER NOT NULL DEFAULT 0,
    iterations              INTEGER NOT NULL DEFAULT 0,
    next_review_at          TEXT NOT NULL,
    last_review_at          TEXT,
    total_reviews           INTEGER NOT NULL DEFAULT 0,
    correct_reviews         INTEGER NOT NULL DEFAULT 0,
    streak                  INTEGER NOT NULL DEFAULT 0,
    introduced_at           TEXT,
    session_reviews         INTEGER NOT NULL DEFAULT 0,
    next_session_review_at  TEXT,
    mastery_level           TEXT NOT NULL DEFAULT 'new'
);

CREATE TABLE IF NOT EXISTS exercise_history (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id          INTEGER NOT NULL REFERENCES word(id) ON DELETE CASCADE,
    exercise_type    TEXT NOT NULL,
    response_time_ms INTEGER NOT NULL DEFAULT 0,
    quality          INTEGER NOT NULL DEFAULT 0 CHECK(quality BETWEEN 0 AND 5),
    was_correct      INTEGER NOT NULL DEFAULT 0,
    user_answer      TEXT,
    session_id       TEXT NOT NULL,
    completed_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_word_progress_next_review ON word_progress(next_review_at);
CREATE INDEX IF NOT EXISTS idx_exercise_history_word ON exercise_history(word_id);
CREATE INDEX IF NOT EXISTS idx_exercise_history_completed ON exercise_history(completed_at);
";
