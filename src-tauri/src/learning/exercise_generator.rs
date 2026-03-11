// src-tauri/src/learning/exercise_generator.rs
//
// Generates concrete exercise payloads for each exercise type.
// All exercises are designed to complete in under 10 seconds.

use anyhow::Result;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::db::{ExerciseType, Word, WordProgress, MasteryLevel};

// ─── Exercise Payloads ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Exercise {
    Introduction(IntroductionExercise),
    MultipleChoice(MultipleChoiceExercise),
    FillInBlank(FillInBlankExercise),
    ContextualGuess(ContextualGuessExercise),
    SpellingCheck(SpellingCheckExercise),
    SynonymMatch(SynonymMatchExercise),
    DefinitionRecall(DefinitionRecallExercise),
    TrueFalse(TrueFalseExercise),
}

impl Exercise {
    pub fn exercise_type(&self) -> ExerciseType {
        match self {
            Self::Introduction(_) => ExerciseType::Introduction,
            Self::MultipleChoice(_) => ExerciseType::MultipleChoice,
            Self::FillInBlank(_) => ExerciseType::FillInBlank,
            Self::ContextualGuess(_) => ExerciseType::ContextualGuess,
            Self::SpellingCheck(_) => ExerciseType::SpellingCheck,
            Self::SynonymMatch(_) => ExerciseType::SynonymMatch,
            Self::DefinitionRecall(_) => ExerciseType::DefinitionRecall,
            Self::TrueFalse(_) => ExerciseType::TrueFalse,
        }
    }

    pub fn word_id(&self) -> i64 {
        match self {
            Self::Introduction(e) => e.word_id,
            Self::MultipleChoice(e) => e.word_id,
            Self::FillInBlank(e) => e.word_id,
            Self::ContextualGuess(e) => e.word_id,
            Self::SpellingCheck(e) => e.word_id,
            Self::SynonymMatch(e) => e.word_id,
            Self::DefinitionRecall(e) => e.word_id,
            Self::TrueFalse(e) => e.word_id,
        }
    }
}

/// Full word presentation: term, phonetic, definition, example, part of speech
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntroductionExercise {
    pub word_id: i64,
    pub term: String,
    pub phonetic: Option<String>,
    pub part_of_speech: String,
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub is_new_word: bool,
}

/// 4-option multiple choice: "Which definition matches [TERM]?"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultipleChoiceExercise {
    pub word_id: i64,
    pub term: String,
    pub question: String,
    pub options: Vec<McOption>,
    pub correct_index: usize,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McOption {
    pub text: String,
    pub is_correct: bool,
}

/// Fill in blank from an example sentence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillInBlankExercise {
    pub word_id: i64,
    pub sentence: String,     // "She showed great _____ in the face of danger."
    pub answer: String,       // "fortitude"
    pub hint: Option<String>, // first letter + length: "f______"
    pub options: Vec<String>, // for guided version
}

/// Given a sentence with the word, identify its meaning
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextualGuessExercise {
    pub word_id: i64,
    pub term: String,
    pub context_sentence: String,
    pub options: Vec<McOption>,
    pub correct_index: usize,
}

/// Type the word from its definition (active recall)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellingCheckExercise {
    pub word_id: i64,
    pub definition: String,
    pub answer: String,
    pub phonetic: Option<String>,
    pub hint: String, // "_ o r t _ t u d e"  (vowels hidden)
}

/// Match the term to its synonym
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynonymMatchExercise {
    pub word_id: i64,
    pub term: String,
    pub options: Vec<McOption>, // includes synonyms and distractors
    pub correct_indices: Vec<usize>,
    pub question: String,
}

/// Show definition → recall the term (hardest exercise)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionRecallExercise {
    pub word_id: i64,
    pub definition: String,
    pub part_of_speech: String,
    pub answer: String,
    pub options: Vec<McOption>, // term recognition mode
    pub correct_index: usize,
}

/// Simple binary: "Does this definition match this word?"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrueFalseExercise {
    pub word_id: i64,
    pub term: String,
    pub shown_definition: String,
    pub is_correct_definition: bool,
    pub explanation: String,
}

// ─── Generator ────────────────────────────────────────────────────────────────

pub struct ExerciseGenerator;

impl ExerciseGenerator {
    /// Choose and generate the most appropriate exercise for this word
    /// based on mastery level and session count.
    pub fn generate(
        word: &Word,
        progress: &WordProgress,
        distractors: &[Word],
        rng: &mut impl Rng,
    ) -> Result<Exercise> {
        let exercise_type = Self::choose_type(progress, rng);
        Self::generate_typed(exercise_type, word, progress, distractors, rng)
    }

    /// Generate a specific exercise type
    pub fn generate_typed(
        exercise_type: ExerciseType,
        word: &Word,
        progress: &WordProgress,
        distractors: &[Word],
        rng: &mut impl Rng,
    ) -> Result<Exercise> {
        Ok(match exercise_type {
            ExerciseType::Introduction => {
                Exercise::Introduction(Self::make_introduction(word, progress))
            }
            ExerciseType::MultipleChoice => {
                Exercise::MultipleChoice(Self::make_multiple_choice(word, distractors, rng))
            }
            ExerciseType::FillInBlank => {
                Exercise::FillInBlank(Self::make_fill_in_blank(word, distractors, rng))
            }
            ExerciseType::ContextualGuess => {
                Exercise::ContextualGuess(Self::make_contextual_guess(word, distractors, rng))
            }
            ExerciseType::SpellingCheck => {
                Exercise::SpellingCheck(Self::make_spelling_check(word))
            }
            ExerciseType::SynonymMatch => {
                Exercise::SynonymMatch(Self::make_synonym_match(word, distractors, rng))
            }
            ExerciseType::DefinitionRecall => {
                Exercise::DefinitionRecall(Self::make_definition_recall(word, distractors, rng))
            }
            ExerciseType::TrueFalse => {
                Exercise::TrueFalse(Self::make_true_false(word, distractors, rng))
            }
        })
    }

    /// Choose exercise type based on mastery progression.
    /// New words get introduction; mastered words get harder recall exercises.
    fn choose_type(progress: &WordProgress, rng: &mut impl Rng) -> ExerciseType {
        match &progress.mastery_level {
            MasteryLevel::New => ExerciseType::Introduction,
            MasteryLevel::Learning => {
                // Alternate between supportive exercises
                let types = [
                    ExerciseType::MultipleChoice,
                    ExerciseType::TrueFalse,
                    ExerciseType::FillInBlank,
                    ExerciseType::ContextualGuess,
                ];
                types.choose(rng).cloned().unwrap()
            }
            MasteryLevel::Reviewing => {
                let types = [
                    ExerciseType::MultipleChoice,
                    ExerciseType::FillInBlank,
                    ExerciseType::ContextualGuess,
                    ExerciseType::SynonymMatch,
                    ExerciseType::DefinitionRecall,
                ];
                types.choose(rng).cloned().unwrap()
            }
            MasteryLevel::Mastered => {
                // Hardest exercises to verify genuine mastery
                let types = [
                    ExerciseType::SpellingCheck,
                    ExerciseType::DefinitionRecall,
                    ExerciseType::SynonymMatch,
                    ExerciseType::ContextualGuess,
                ];
                types.choose(rng).cloned().unwrap()
            }
        }
    }

    // ── Exercise Builders ─────────────────────────────────────────────────

    fn make_introduction(word: &Word, progress: &WordProgress) -> IntroductionExercise {
        let example = word.examples.first().cloned();
        IntroductionExercise {
            word_id: word.id,
            term: word.term.clone(),
            phonetic: word.phonetic.clone(),
            part_of_speech: word.part_of_speech.clone(),
            definition: word.definition.clone(),
            example,
            synonyms: word.synonyms.iter().take(3).cloned().collect(),
            is_new_word: progress.total_reviews == 0,
        }
    }

    fn make_multiple_choice(word: &Word, distractors: &[Word], rng: &mut impl Rng) -> MultipleChoiceExercise {
        let correct_def = word.definition.clone();
        let mut options: Vec<McOption> = vec![McOption {
            text: correct_def,
            is_correct: true,
        }];

        // Add distractor definitions
        for d in distractors.iter().take(3) {
            options.push(McOption {
                text: d.definition.clone(),
                is_correct: false,
            });
        }
        options.shuffle(rng);

        let correct_index = options.iter().position(|o| o.is_correct).unwrap_or(0);

        MultipleChoiceExercise {
            word_id: word.id,
            term: word.term.clone(),
            question: format!("What does \"{}\" mean?", word.term),
            options,
            correct_index,
            hint: word.part_of_speech.clone().into(),
        }
    }

    fn make_fill_in_blank(word: &Word, distractors: &[Word], rng: &mut impl Rng) -> FillInBlankExercise {
        let sentence = word
            .examples
            .first()
            .cloned()
            .unwrap_or_else(|| format!("He demonstrated great {} in his work.", word.term));

        // Replace the word with blank
        let blank_sentence = sentence.replace(&word.term, "_____");

        let hint = build_hint(&word.term);

        // Provide 4 options including the correct term
        let mut options = vec![word.term.clone()];
        for d in distractors.iter().take(3) {
            options.push(d.term.clone());
        }
        options.shuffle(rng);

        FillInBlankExercise {
            word_id: word.id,
            sentence: blank_sentence,
            answer: word.term.clone(),
            hint: Some(hint),
            options,
        }
    }

    fn make_contextual_guess(word: &Word, distractors: &[Word], rng: &mut impl Rng) -> ContextualGuessExercise {
        let context_sentence = word
            .examples
            .first()
            .cloned()
            .unwrap_or_else(|| format!("She showed remarkable {} when faced with difficulty.", word.term));

        let mut options = vec![McOption {
            text: word.definition.clone(),
            is_correct: true,
        }];
        for d in distractors.iter().take(3) {
            options.push(McOption {
                text: d.definition.clone(),
                is_correct: false,
            });
        }
        options.shuffle(rng);
        let correct_index = options.iter().position(|o| o.is_correct).unwrap_or(0);

        ContextualGuessExercise {
            word_id: word.id,
            term: word.term.clone(),
            context_sentence,
            options,
            correct_index,
        }
    }

    fn make_spelling_check(word: &Word) -> SpellingCheckExercise {
        let hint = build_hint(&word.term);
        SpellingCheckExercise {
            word_id: word.id,
            definition: word.definition.clone(),
            answer: word.term.clone(),
            phonetic: word.phonetic.clone(),
            hint,
        }
    }

    fn make_synonym_match(word: &Word, distractors: &[Word], rng: &mut impl Rng) -> SynonymMatchExercise {
        let synonyms = &word.synonyms;
        let has_synonyms = !synonyms.is_empty();

        let mut options: Vec<McOption> = if has_synonyms {
            synonyms.iter().take(2).map(|s| McOption { text: s.clone(), is_correct: true }).collect()
        } else {
            vec![] // fallback: no synonyms known
        };

        // Add distractor terms
        for d in distractors.iter().take(4) {
            let is_synonym = synonyms.contains(&d.term);
            options.push(McOption {
                text: d.term.clone(),
                is_correct: is_synonym,
            });
        }
        options.shuffle(rng);

        let correct_indices: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.is_correct)
            .map(|(i, _)| i)
            .collect();

        SynonymMatchExercise {
            word_id: word.id,
            term: word.term.clone(),
            options,
            correct_indices,
            question: format!("Which word is a synonym of \"{}\"?", word.term),
        }
    }

    fn make_definition_recall(word: &Word, distractors: &[Word], rng: &mut impl Rng) -> DefinitionRecallExercise {
        let mut options = vec![McOption {
            text: word.term.clone(),
            is_correct: true,
        }];
        for d in distractors.iter().take(3) {
            options.push(McOption {
                text: d.term.clone(),
                is_correct: false,
            });
        }
        options.shuffle(rng);
        let correct_index = options.iter().position(|o| o.is_correct).unwrap_or(0);

        DefinitionRecallExercise {
            word_id: word.id,
            definition: word.definition.clone(),
            part_of_speech: word.part_of_speech.clone(),
            answer: word.term.clone(),
            options,
            correct_index,
        }
    }

    fn make_true_false(word: &Word, distractors: &[Word], rng: &mut impl Rng) -> TrueFalseExercise {
        let is_correct_definition = rng.gen_bool(0.5);

        let (shown_definition, explanation) = if is_correct_definition {
            (
                word.definition.clone(),
                format!("✓ Correct! \"{}\" means: {}", word.term, word.definition),
            )
        } else {
            // Show a wrong definition from a random distractor
            let wrong_def = distractors
                .first()
                .map(|d| d.definition.clone())
                .unwrap_or_else(|| "the opposite of what it seems".to_string());
            (
                wrong_def,
                format!("✗ Wrong! \"{}\" actually means: {}", word.term, word.definition),
            )
        };

        TrueFalseExercise {
            word_id: word.id,
            term: word.term.clone(),
            shown_definition,
            is_correct_definition,
            explanation,
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Build a hint showing consonants, hiding vowels: "f_rt_t_d_"
fn build_hint(term: &str) -> String {
    term.chars()
        .map(|c| {
            if "aeiou".contains(c.to_ascii_lowercase()) {
                '_'
            } else {
                c
            }
        })
        .collect()
}
