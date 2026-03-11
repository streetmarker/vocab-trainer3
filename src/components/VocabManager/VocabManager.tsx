// src/components/VocabManager/VocabManager.tsx
import React, { useEffect, useState } from "react";
import type { Word, PartOfSpeech } from "../../types";
import { DIFFICULTY_LABELS, DIFFICULTY_COLORS } from "../../types";
import { api } from "../../hooks/useTauri";
import "./VocabManager.css";

const POS_OPTIONS: PartOfSpeech[] = [
  "noun", "verb", "adjective", "adverb", "pronoun",
  "preposition", "conjunction", "interjection", "phrase"
];

interface WordFormData {
  term: string;
  definition: string;
  partOfSpeech: PartOfSpeech;
  phonetic: string;
  examples: string;
  synonyms: string;
  antonyms: string;
  tags: string;
  difficulty: number;
}

const emptyForm: WordFormData = {
  term: "", definition: "", partOfSpeech: "noun", phonetic: "",
  examples: "", synonyms: "", antonyms: "", tags: "", difficulty: 2,
};

export const VocabManager: React.FC = () => {
  const [words, setWords] = useState<Word[]>([]);
  const [search, setSearch] = useState("");
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<WordFormData>(emptyForm);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [seeding, setSeeding] = useState(false);

  const loadWords = async () => {
    const w = await api.getWords();
    setWords(w);
  };

  useEffect(() => { loadWords(); }, []);

  const filtered = words.filter(
    (w) =>
      w.term.toLowerCase().includes(search.toLowerCase()) ||
      w.definition.toLowerCase().includes(search.toLowerCase()) ||
      w.tags.some((t) => t.toLowerCase().includes(search.toLowerCase()))
  );

  const handleSave = async () => {
    if (!form.term.trim() || !form.definition.trim()) {
      setError("Term and definition are required.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      await api.addWord({
        term: form.term.trim(),
        definition: form.definition.trim(),
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
    if (!confirm("Remove this word from your library?")) return;
    await api.deleteWord(id);
    await loadWords();
  };

  const handleSeed = async () => {
    setSeeding(true);
    const count = await api.seedSampleWords();
    await loadWords();
    setSeeding(false);
    alert(`Added ${count} sample words!`);
  };

  return (
    <div className="vocab-manager">
      {/* ── Toolbar ─────────────────────────────────────────────────────── */}
      <div className="vm-toolbar">
        <div className="vm-search-wrap">
          <span className="search-icon">🔍</span>
          <input
            className="vm-search"
            placeholder="Search words, definitions, tags…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          {search && (
            <button className="search-clear" onClick={() => setSearch("")}>✕</button>
          )}
        </div>
        <div className="vm-actions">
          <button className="btn-secondary" onClick={handleSeed} disabled={seeding}>
            {seeding ? "Adding…" : "Load Samples"}
          </button>
          <button className="btn-primary" onClick={() => setShowForm(true)}>
            + Add Word
          </button>
        </div>
      </div>

      {/* ── Stats row ───────────────────────────────────────────────────── */}
      <div className="vm-stats">
        <span>{words.length} words total</span>
        <span>·</span>
        <span>{filtered.length} shown</span>
        {search && <span className="vm-filter-tag">filtered by "{search}"</span>}
      </div>

      {/* ── Word List ────────────────────────────────────────────────────── */}
      {filtered.length === 0 ? (
        <div className="vm-empty">
          <div className="empty-icon">📚</div>
          <p>
            {search ? `No words matching "${search}"` : "Your vocabulary library is empty."}
          </p>
          {!search && (
            <button className="btn-primary" onClick={() => setShowForm(true)}>
              Add your first word
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
              <h2>Add New Word</h2>
              <button className="modal-close" onClick={() => setShowForm(false)}>✕</button>
            </div>

            <div className="form-grid">
              <div className="form-field span-2">
                <label>Term *</label>
                <input
                  value={form.term}
                  onChange={(e) => setForm({ ...form, term: e.target.value })}
                  placeholder="e.g., ephemeral"
                  autoFocus
                />
              </div>

              <div className="form-field">
                <label>Part of Speech</label>
                <select
                  value={form.partOfSpeech}
                  onChange={(e) => setForm({ ...form, partOfSpeech: e.target.value as PartOfSpeech })}
                >
                  {POS_OPTIONS.map((p) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </div>

              <div className="form-field">
                <label>Phonetic</label>
                <input
                  value={form.phonetic}
                  onChange={(e) => setForm({ ...form, phonetic: e.target.value })}
                  placeholder="/ɪˈfem.ər.əl/"
                />
              </div>

              <div className="form-field span-2">
                <label>Definition *</label>
                <textarea
                  value={form.definition}
                  onChange={(e) => setForm({ ...form, definition: e.target.value })}
                  placeholder="Clear, concise definition"
                  rows={2}
                />
              </div>

              <div className="form-field span-2">
                <label>Examples (one per line)</label>
                <textarea
                  value={form.examples}
                  onChange={(e) => setForm({ ...form, examples: e.target.value })}
                  placeholder="The ephemeral beauty of cherry blossoms..."
                  rows={2}
                />
              </div>

              <div className="form-field">
                <label>Synonyms (comma separated)</label>
                <input
                  value={form.synonyms}
                  onChange={(e) => setForm({ ...form, synonyms: e.target.value })}
                  placeholder="fleeting, transient"
                />
              </div>

              <div className="form-field">
                <label>Antonyms (comma separated)</label>
                <input
                  value={form.antonyms}
                  onChange={(e) => setForm({ ...form, antonyms: e.target.value })}
                  placeholder="permanent, enduring"
                />
              </div>

              <div className="form-field">
                <label>Tags (comma separated)</label>
                <input
                  value={form.tags}
                  onChange={(e) => setForm({ ...form, tags: e.target.value })}
                  placeholder="literary, gre, common"
                />
              </div>

              <div className="form-field">
                <label>Difficulty: <span style={{ color: DIFFICULTY_COLORS[form.difficulty] }}>
                  {DIFFICULTY_LABELS[form.difficulty]}
                </span></label>
                <input
                  type="range"
                  min={1} max={5} step={1}
                  value={form.difficulty}
                  onChange={(e) => setForm({ ...form, difficulty: +e.target.value })}
                  className="difficulty-slider"
                />
                <div className="slider-labels">
                  <span>Easy</span><span>Hard</span>
                </div>
              </div>
            </div>

            {error && <div className="form-error">{error}</div>}

            <div className="modal-footer">
              <button className="btn-ghost" onClick={() => setShowForm(false)}>Cancel</button>
              <button className="btn-primary" onClick={handleSave} disabled={saving}>
                {saving ? "Saving…" : "Add Word"}
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
          <span className="vm-pos">{word.partOfSpeech}</span>
          {word.phonetic && <span className="vm-phonetic">{word.phonetic}</span>}
        </div>
        <div className="vm-row-center">
          <span className="vm-def-preview">{word.definition}</span>
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
            aria-label="Delete"
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
              <span className="detail-label">Examples</span>
              {word.examples.map((ex, i) => (
                <div key={i} className="detail-example">"{ex}"</div>
              ))}
            </div>
          )}
          {word.synonyms.length > 0 && (
            <div className="detail-section">
              <span className="detail-label">Synonyms</span>
              <div className="chip-row">
                {word.synonyms.map((s) => (
                  <span key={s} className="chip chip-synonym">{s}</span>
                ))}
              </div>
            </div>
          )}
          {word.antonyms.length > 0 && (
            <div className="detail-section">
              <span className="detail-label">Antonyms</span>
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
