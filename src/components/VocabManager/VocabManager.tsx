// src/components/VocabManager/VocabManager.tsx
import React, { useEffect, useState } from "react";
import type { Word, PartOfSpeech } from "../../types";
import { DIFFICULTY_LABELS, DIFFICULTY_COLORS, PART_OF_SPEECH_LABELS } from "../../types";
import { api } from "../../hooks/useTauri";
import { ImportWords } from "../ImportWords/ImportWords";
import "./VocabManager.css";

const POS_OPTIONS: PartOfSpeech[] = [
  "noun", "verb", "adjective", "adverb", "pronoun",
  "preposition", "conjunction", "interjection", "phrase"
];

interface WordFormData {
  term: string;
  definition: string;
  definitionPl: string;
  partOfSpeech: PartOfSpeech;
  phonetic: string;
  examples: string;
  synonyms: string;
  antonyms: string;
  tags: string;
  difficulty: number;
}

const emptyForm: WordFormData = {
  term: "", definition: "", definitionPl: "", partOfSpeech: "noun", phonetic: "",
  examples: "", synonyms: "", antonyms: "", tags: "", difficulty: 2,
};

export const VocabManager: React.FC = () => {
  const [words, setWords]     = useState<Word[]>([]);
  const [search, setSearch]   = useState("");
  const [showForm, setShowForm] = useState(false);
  const [form, setForm]       = useState<WordFormData>(emptyForm);
  const [saving, setSaving]   = useState(false);
  const [error, setError]     = useState("");
  const [seeding, setSeeding] = useState(false);

  const loadWords = async () => { setWords(await api.getWords()); };
  useEffect(() => { loadWords(); }, []);

  const filtered = words.filter(
    (w) =>
      w.term.toLowerCase().includes(search.toLowerCase()) ||
      w.definition.toLowerCase().includes(search.toLowerCase()) ||
      w.definitionPl?.toLowerCase().includes(search.toLowerCase()) ||
      w.tags.some((t) => t.toLowerCase().includes(search.toLowerCase()))
  );

  const handleSave = async () => {
    if (!form.term.trim() || !form.definition.trim()) {
      setError("Słowo i definicja są wymagane.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      await api.addWord({
        term: form.term.trim(),
        definition: form.definition.trim(),
        definitionPl: form.definitionPl.trim() || undefined,
        partOfSpeech: form.partOfSpeech,
        phonetic: form.phonetic.trim() || undefined,
        examples: form.examples.split("\n").map((s) => s.trim()).filter(Boolean),
        synonyms: form.synonyms.split(",").map((s) => s.trim()).filter(Boolean),
        antonyms: form.antonyms.split(",").map((s) => s.trim()).filter(Boolean),
        tags: form.tags.split(",").map((s) => s.trim()).filter(Boolean),
        difficulty: form.difficulty,
      });
      setForm(emptyForm);
      setShowForm(false);
      await loadWords();
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm("Usunąć to słowo z biblioteki?")) return;
    await api.deleteWord(id);
    await loadWords();
  };

  const handleSeed = async () => {
    setSeeding(true);
    const count = await api.seedSampleWords();
    await loadWords();
    setSeeding(false);
    alert(`Dodano ${count} przykładowych słów!`);
  };

  return (
    <div className="vocab-manager">

      {/* ── Toolbar ─────────────────────────────────────────────────────── */}
      <div className="dash-header">
        <div>
          <h1 className="dash-title">Słownictwo</h1>
          <p className="dash-subtitle">Zarządzaj swoją biblioteką angielskich słów</p>
        </div>
      </div>

      <div className="vm-toolbar">
        <div className="vm-search-wrap">
          <span className="search-icon">🔍</span>
          <input
            className="vm-search"
            placeholder="Szukaj słów, definicji, tagów…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          {search && (
            <button className="search-clear" onClick={() => setSearch("")}>✕</button>
          )}
        </div>
        <div className="vm-actions">
          <ImportWords onImportDone={loadWords} />
          <button className="btn-secondary" onClick={handleSeed} disabled={seeding}>
            {seeding ? "Dodawanie…" : "Załaduj przykłady"}
          </button>
          <button className="btn-primary" onClick={() => setShowForm(true)}>
            + Dodaj słowo
          </button>
        </div>
      </div>

      {/* ── Stats row ───────────────────────────────────────────────────── */}
      <div className="vm-stats">
        <span>{words.length} {words.length === 1 ? "słowo" : "słów"} łącznie</span>
        <span>·</span>
        <span>{filtered.length} wyświetlanych</span>
        {search && <span className="vm-filter-tag">filtr: „{search}"</span>}
      </div>

      {/* ── Word List ────────────────────────────────────────────────────── */}
      {filtered.length === 0 ? (
        <div className="vm-empty">
          <div className="empty-icon">📚</div>
          <p>
            {search
              ? `Brak słów pasujących do „${search}"`
              : "Biblioteka jest pusta."}
          </p>
          {!search && (
            <button className="btn-primary" onClick={() => setShowForm(true)}>
              Dodaj pierwsze słowo
            </button>
          )}
        </div>
      ) : (
        <div className="vm-list">
          {filtered.map((word) => (
            <VocabRow key={word.id} word={word} onDelete={handleDelete} />
          ))}
        </div>
      )}

      {/* ── Add Word Modal ───────────────────────────────────────────────── */}
      {showForm && (
        <div className="modal-overlay" onClick={() => setShowForm(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Dodaj nowe słowo</h2>
              <button className="modal-close" onClick={() => setShowForm(false)}>✕</button>
            </div>

            <div className="form-grid">
              <div className="form-field span-2">
                <label>Słowo (angielski) *</label>
                <input
                  value={form.term}
                  onChange={(e) => setForm({ ...form, term: e.target.value })}
                  placeholder="np. ephemeral"
                  autoFocus
                />
              </div>

              <div className="form-field">
                <label>Część mowy</label>
                <select
                  value={form.partOfSpeech}
                  onChange={(e) => setForm({ ...form, partOfSpeech: e.target.value as PartOfSpeech })}
                >
                  {POS_OPTIONS.map((p) => (
                    <option key={p} value={p}>{PART_OF_SPEECH_LABELS[p] ?? p}</option>
                  ))}
                </select>
              </div>

              <div className="form-field">
                <label>Wymowa (fonetyczna)</label>
                <input
                  value={form.phonetic}
                  onChange={(e) => setForm({ ...form, phonetic: e.target.value })}
                  placeholder="/ɪˈfem.ər.əl/"
                />
              </div>

              <div className="form-field span-2">
                <label>Definicja (angielski) *</label>
                <textarea
                  value={form.definition}
                  onChange={(e) => setForm({ ...form, definition: e.target.value })}
                  placeholder="Zwięzła definicja po angielsku"
                  rows={2}
                />
              </div>

              <div className="form-field span-2">
                <label>
                  <span className="pl-flag">🇵🇱</span> Wyjaśnienie po polsku
                </label>
                <textarea
                  value={form.definitionPl}
                  onChange={(e) => setForm({ ...form, definitionPl: e.target.value })}
                  placeholder="Tłumaczenie lub opis po polsku, np. 'Krótkotrwały, przemijający — coś, co istnieje tylko przez chwilę'"
                  rows={2}
                />
              </div>

              <div className="form-field span-2">
                <label>Przykłady użycia (jeden na linię)</label>
                <textarea
                  value={form.examples}
                  onChange={(e) => setForm({ ...form, examples: e.target.value })}
                  placeholder="The ephemeral beauty of cherry blossoms..."
                  rows={2}
                />
              </div>

              <div className="form-field">
                <label>Synonimy (oddzielone przecinkami)</label>
                <input
                  value={form.synonyms}
                  onChange={(e) => setForm({ ...form, synonyms: e.target.value })}
                  placeholder="fleeting, transient"
                />
              </div>

              <div className="form-field">
                <label>Antonimy (oddzielone przecinkami)</label>
                <input
                  value={form.antonyms}
                  onChange={(e) => setForm({ ...form, antonyms: e.target.value })}
                  placeholder="permanent, enduring"
                />
              </div>

              <div className="form-field">
                <label>Tagi (oddzielone przecinkami)</label>
                <input
                  value={form.tags}
                  onChange={(e) => setForm({ ...form, tags: e.target.value })}
                  placeholder="literackie, egzamin, codzienne"
                />
              </div>

              <div className="form-field">
                <label>
                  Poziom trudności:{" "}
                  <span style={{ color: DIFFICULTY_COLORS[form.difficulty] }}>
                    {DIFFICULTY_LABELS[form.difficulty]}
                  </span>
                </label>
                <input
                  type="range"
                  min={1} max={5} step={1}
                  value={form.difficulty}
                  onChange={(e) => setForm({ ...form, difficulty: +e.target.value })}
                  className="difficulty-slider"
                />
                <div className="slider-labels">
                  <span>Łatwy</span><span>Trudny</span>
                </div>
              </div>
            </div>

            {error && <div className="form-error">{error}</div>}

            <div className="modal-footer">
              <button className="btn-ghost" onClick={() => setShowForm(false)}>Anuluj</button>
              <button className="btn-primary" onClick={handleSave} disabled={saving}>
                {saving ? "Zapisywanie…" : "Dodaj słowo"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

// ─── Vocab Row ────────────────────────────────────────────────────────────────

const VocabRow: React.FC<{ word: Word; onDelete: (id: number) => void }> = ({ word, onDelete }) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className={`vm-row ${expanded ? "expanded" : ""}`}>
      <div className="vm-row-main" onClick={() => setExpanded(!expanded)}>
        <div className="vm-row-left">
          <span className="vm-term">{word.term}</span>
          <span className="vm-pos">{PART_OF_SPEECH_LABELS[word.partOfSpeech] ?? word.partOfSpeech}</span>
          {word.phonetic && <span className="vm-phonetic">{word.phonetic}</span>}
        </div>
        <div className="vm-row-center">
          <span className="vm-def-preview">{word.definition}</span>
          {word.definitionPl && (
            <span className="vm-def-pl-preview">
              <span className="pl-flag">🇵🇱</span>{word.definitionPl}
            </span>
          )}
        </div>
        <div className="vm-row-right">
          <span
            className="vm-diff-badge"
            style={{
              background: `${DIFFICULTY_COLORS[word.difficulty]}20`,
              color: DIFFICULTY_COLORS[word.difficulty],
              borderColor: `${DIFFICULTY_COLORS[word.difficulty]}40`,
            }}
          >
            {DIFFICULTY_LABELS[word.difficulty]}
          </span>
          <button
            className="vm-delete"
            onClick={(e) => { e.stopPropagation(); onDelete(word.id); }}
            aria-label="Usuń"
          >
            🗑
          </button>
          <span className="vm-expand-icon">{expanded ? "▲" : "▼"}</span>
        </div>
      </div>

      {expanded && (
        <div className="vm-row-detail">
          {word.examples.length > 0 && (
            <div className="detail-section">
              <span className="detail-label">Przykłady</span>
              {word.examples.map((ex, i) => (
                <div key={i} className="detail-example">„{ex}"</div>
              ))}
            </div>
          )}
          {word.synonyms.length > 0 && (
            <div className="detail-section">
              <span className="detail-label">Synonimy</span>
              <div className="chip-row">
                {word.synonyms.map((s) => (
                  <span key={s} className="chip chip-synonym">{s}</span>
                ))}
              </div>
            </div>
          )}
          {word.antonyms.length > 0 && (
            <div className="detail-section">
              <span className="detail-label">Antonimy</span>
              <div className="chip-row">
                {word.antonyms.map((a) => (
                  <span key={a} className="chip chip-antonym">{a}</span>
                ))}
              </div>
            </div>
          )}
          {word.tags.length > 0 && (
            <div className="chip-row">
              {word.tags.map((t) => (
                <span key={t} className="chip chip-tag">#{t}</span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
