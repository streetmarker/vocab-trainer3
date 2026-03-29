// src/components/VocabManager/VocabManager.tsx
//
// Extends the word list with SRS overview:
//   - SrsToday panel at the top (daily stats + mastery progress bar)
//   - View toggle: flat list ↔ SRS-grouped list
//   - SrsBadge on every row showing mastery level + interval + streak
//
// No new screens — everything is added inline.

import React, { useEffect, useState } from "react";
import type { PartOfSpeech } from "../../types";
import type { WordWithProgress, SrsOverview } from "../../hooks/useTauri";
import { DIFFICULTY_LABELS, DIFFICULTY_COLORS, PART_OF_SPEECH_LABELS } from "../../types";
import { api } from "../../hooks/useTauri";
import { ImportWords } from "../ImportWords/ImportWords";
import { CategorizationAgent, AgentStatus } from "../../utils/CategorizationAgent";
import { SrsToday } from "../SrsPanel/SrsToday";
import { SrsGroupHeader } from "../SrsPanel/SrsGroupHeader";
import { SrsBadge } from "../SrsPanel/SrsBadge";
import { SRS_GROUPS } from "../SrsPanel/srs-config";
import "../SrsPanel/SrsPanel.css";
import "./VocabManager.css";

const POS_OPTIONS: PartOfSpeech[] = [
  "noun","verb","adjective","adverb","pronoun",
  "preposition","conjunction","interjection","phrase",
];

interface WordFormData {
  term: string; definition: string; definitionPl: string;
  partOfSpeech: PartOfSpeech; phonetic: string;
  examples: string; synonyms: string; antonyms: string;
  tags: string; difficulty: number;
  sentencePl: string; sentenceEn: string;
  category: string;
}
const emptyForm: WordFormData = {
  term:"",definition:"",definitionPl:"",partOfSpeech:"noun",phonetic:"",
  examples:"",synonyms:"",antonyms:"",tags:"",difficulty:2,
  sentencePl:"",sentenceEn:"",category:"bez kategorii",
};

type ViewMode = "flat" | "grouped" | "category";

export const VocabManager: React.FC<{ activeCategory: string }> = ({ activeCategory }) => {
  const [overview, setOverview]   = useState<SrsOverview | null>(null);
  const [srsLoading, setSrsLoading] = useState(true);
  const [search, setSearch]       = useState("");
  const [viewMode, setViewMode]   = useState<ViewMode>("grouped");
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [showForm, setShowForm]   = useState(false);
  const [form, setForm]           = useState<WordFormData>(emptyForm);
  const [saving, setSaving]       = useState(false);
  const [error, setError]         = useState("");
  const [seeding, setSeeding]     = useState(false);
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);

  const loadData = async () => {
    setSrsLoading(true);
    try {
      const ov = await api.getSrsOverview();
      setOverview(ov);
    } finally {
      setSrsLoading(false);
    }
  };

  useEffect(() => { 
    loadData(); 
    const agent = CategorizationAgent.getInstance();
    agent.setStatusListener(setAgentStatus);
  }, []);

  const words: WordWithProgress[] = overview?.words ?? [];

  // Filtrowanie najpierw po aktywnej kategorii z sidebara, potem po wyszukiwarce
  const categoryFiltered = words.filter(w =>
    activeCategory === "Wszystkie" || w.category === activeCategory
  );

  const filtered = categoryFiltered.filter(w =>
    w.term.toLowerCase().includes(search.toLowerCase()) ||
    w.definition.toLowerCase().includes(search.toLowerCase()) ||
    (w.definitionPl ?? "").toLowerCase().includes(search.toLowerCase()) ||
    w.tags.some(t => t.toLowerCase().includes(search.toLowerCase()))
  );

  const toggleGroup = (id: string) =>
    setCollapsed(c => ({ ...c, [id]: !c[id] }));

  const handleSave = async () => {
    if (!form.term.trim() || !form.definition.trim()) {
      setError("Słowo i definicja są wymagane."); return;
    }
    setSaving(true); setError("");
    try {
      await api.addWord({
        term: form.term.trim(), definition: form.definition.trim(),
        definitionPl: form.definitionPl.trim() || undefined,
        partOfSpeech: form.partOfSpeech,
        phonetic: form.phonetic.trim() || undefined,
        examples: form.examples.split("\n").map(s=>s.trim()).filter(Boolean),
        synonyms: form.synonyms.split(",").map(s=>s.trim()).filter(Boolean),
        antonyms: form.antonyms.split(",").map(s=>s.trim()).filter(Boolean),
        tags: form.tags.split(",").map(s=>s.trim()).filter(Boolean),
        difficulty: form.difficulty,
        sentencePl: form.sentencePl.trim() || undefined,
        sentenceEn: form.sentenceEn.trim() || undefined,
        category: form.category.trim() || "bez kategorii",
      });
      setForm(emptyForm); setShowForm(false); await loadData();
      window.dispatchEvent(new CustomEvent("refresh-categories"));
    } catch (e: any) { setError(e.toString()); }
    finally { setSaving(false); }
  };

  const handleDelete = async (id: number) => {
    if (!confirm("Usunąć to słowo z biblioteki?")) return;
    await api.deleteWord(id); await loadData();
  };

  const handleReclassify = async () => {
    const agent = CategorizationAgent.getInstance();
    await agent.run();
    await loadData();
    window.dispatchEvent(new CustomEvent("refresh-categories"));
  };

  return (
    <div className="vocab-manager">

      {/* ── Header ─────────────────────────────────────────────── */}
      <div className="dash-header">
        <div>
          <h1 className="dash-title">Słownictwo</h1>
          <p className="dash-subtitle">Zarządzaj swoją biblioteką angielskich słów</p>
        </div>
      </div>

      {/* ── SRS Today Panel ────────────────────────────────────── */}
      <SrsToday stats={overview?.today ?? {
        dueToday:0, newWords:0, learning:0, reviewing:0, mastered:0, total:0,
      }} loading={srsLoading} />

      {/* ── Toolbar ────────────────────────────────────────────── */}
      <div className="vm-toolbar">
        <div className="vm-search-wrap">
          <span className="search-icon">🔍</span>
          <input
            className="vm-search"
            placeholder="Szukaj słów, definicji, tagów…"
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
          {search && <button className="search-clear" onClick={() => setSearch("")}>✕</button>}
        </div>
        <div className="vm-actions">
          {/* View mode toggle */}
          <div className="vm-view-toggle">
            <button
              className={`vm-view-btn ${viewMode === "grouped" ? "vm-view-btn--active" : ""}`}
              onClick={() => setViewMode("grouped")} title="Grupuj według SRS"
            >
              ≡ Grupy
            </button>
            <button
              className={`vm-view-btn ${viewMode === "category" ? "vm-view-btn--active" : ""}`}
              onClick={() => setViewMode("category")} title="Grupuj według kategorii"
            >
              📁 Kategorie
            </button>
            <button
              className={`vm-view-btn ${viewMode === "flat" ? "vm-view-btn--active" : ""}`}
              onClick={() => setViewMode("flat")} title="Lista płaska"
            >
              ☰ Lista
            </button>
          </div>
          <ImportWords onImportDone={loadData} />
          <button className="btn-secondary" onClick={handleReclassify} disabled={saving}>
            🤖 Kategoryzuj AI
          </button>
          <button className="btn-primary" onClick={() => setShowForm(true)}>
            + Dodaj słowo
          </button>
        </div>
      </div>

      {/* ── Stats row ───────────────────────────────────────────── */}
      <div className="vm-stats">
        <span>{words.length} {words.length===1 ? "słowo":"słów"} łącznie</span>
        <span>·</span>
        <span>{filtered.length} wyświetlanych</span>
        {search && <span className="vm-filter-tag">filtr: „{search}"</span>}
      </div>

      {/* ── Agent Progress Overlay ────────────────────────────────────── */}
      {agentStatus?.isProcessing && (
        <div className="agent-progress-overlay">
          <div className="agent-card">
            <div className="agent-header">
              <span className="agent-bot-icon">🤖</span>
              <div>
                <div className="agent-name">Agent Kategoryzacji</div>
                <div className="agent-status">Analizowanie słownictwa...</div>
              </div>
            </div>
            <div className="agent-body">
              <div className="agent-current-word">
                Przetwarzanie: <strong>{agentStatus.currentWord}</strong>
              </div>
              <div className="agent-progress-bar-wrap">
                <div 
                  className="agent-progress-fill" 
                  style={{ width: `${(agentStatus.progress / agentStatus.total) * 100}%` }} 
                />
              </div>
              <div className="agent-count">
                {agentStatus.progress} / {agentStatus.total} słów
              </div>
            </div>
          </div>
        </div>
      )}

      {/* ── Word List ────────────────────────────────────────────── */}
      {filtered.length === 0 ? (
        <div className="vm-empty">
          <div className="empty-icon">📚</div>
          <p>{search ? `Brak słów pasujących do „${search}"` : "Biblioteka jest pusta."}</p>
          {!search && (
            <button className="btn-primary" onClick={() => setShowForm(true)}>
              Dodaj pierwsze słowo
            </button>
          )}
        </div>
      ) : viewMode === "flat" ? (
        <div className="vm-list">
          {filtered.map(w => <VocabRow key={w.id} word={w} onDelete={handleDelete} />)}
        </div>
      ) : viewMode === "category" ? (
        <div className="vm-list">
          {Array.from(new Set(filtered.map(w => w.category || "bez kategorii"))).sort().map(cat => {
            const groupWords = filtered.filter(w => (w.category || "bez kategorii") === cat);
            const isCollapsed = !!collapsed[cat];
            return (
              <div key={cat} className="srs-group">
                <SrsGroupHeader
                  icon="📁"
                  label={cat}
                  color="#a78bfa"
                  words={groupWords}
                  collapsed={isCollapsed}
                  onToggle={() => toggleGroup(cat)}
                />
                {!isCollapsed && groupWords.map(w =>
                  <VocabRow key={w.id} word={w} onDelete={handleDelete} />
                )}
              </div>
            );
          })}
        </div>
      ) : (
        // ── Grouped view ──────────────────────────────────────────
        <div className="vm-list">
          {SRS_GROUPS.map(group => {
            const groupWords = filtered.filter(group.filter as (w: WordWithProgress) => boolean);
            if (groupWords.length === 0) return null;
            const isCollapsed = !!collapsed[group.id];
            return (
              <div key={group.id} className="srs-group">
                <SrsGroupHeader
                  icon={group.icon}
                  label={group.label}
                  color={group.color}
                  words={groupWords}
                  collapsed={isCollapsed}
                  onToggle={() => toggleGroup(group.id)}
                />
                {!isCollapsed && groupWords.map(w =>
                  <VocabRow key={w.id} word={w} onDelete={handleDelete} />
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* ── Add Word Modal ───────────────────────────────────────── */}
      {showForm && (
        <div className="modal-overlay" onClick={() => setShowForm(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Dodaj nowe słowo</h2>
              <button className="modal-close" onClick={() => setShowForm(false)}>✕</button>
            </div>
            <div className="form-grid">
              <div className="form-field span-2">
                <label>Słowo (angielski) *</label>
                <input value={form.term} onChange={e => setForm({...form, term:e.target.value})}
                  placeholder="np. ephemeral" autoFocus />
              </div>
              <div className="form-field">
                <label>Część mowy</label>
                <select value={form.partOfSpeech}
                  onChange={e => setForm({...form, partOfSpeech:e.target.value as PartOfSpeech})}>
                  {POS_OPTIONS.map(p => <option key={p} value={p}>{PART_OF_SPEECH_LABELS[p]??p}</option>)}
                </select>
              </div>
              <div className="form-field">
                <label>Wymowa (fonetyczna)</label>
                <input value={form.phonetic} onChange={e => setForm({...form, phonetic:e.target.value})}
                  placeholder="/ɪˈfem.ər.əl/" />
              </div>
              <div className="form-field">
                <label>Kategoria</label>
                <input value={form.category} onChange={e => setForm({...form, category:e.target.value})}
                  placeholder="np. IT, Business, Codzienne" />
              </div>
              <div className="form-field span-2">
                <label>Definicja (angielski) *</label>
                <textarea value={form.definition} onChange={e => setForm({...form, definition:e.target.value})}
                  placeholder="Zwięzła definicja po angielsku" rows={2} />
              </div>
              <div className="form-field span-2">
                <label><span className="pl-flag">🇵🇱</span> Wyjaśnienie po polsku</label>
                <textarea value={form.definitionPl} onChange={e => setForm({...form, definitionPl:e.target.value})}
                  placeholder="Tłumaczenie lub opis po polsku" rows={2} />
              </div>
              <div className="form-field span-2">
                <label>🇵🇱 Zdanie po polsku <span className="form-label-hint">(słowo angielskie zostanie pogrubione na fiszce)</span></label>
                <textarea value={form.sentencePl} onChange={e => setForm({...form, sentencePl:e.target.value})}
                  placeholder={"np. Jego ephemeral piękno kwiatów wiśni jest niezapomniane"} rows={2} />
              </div>
              <div className="form-field span-2">
                <label>🇬🇧 Zdanie po angielsku <span className="form-label-hint">(słowo zostanie pogrubione na fiszce)</span></label>
                <textarea value={form.sentenceEn} onChange={e => setForm({...form, sentenceEn:e.target.value})}
                  placeholder={"np. The ephemeral beauty of cherry blossoms reminds us to cherish the moment."} rows={2} />
              </div>
              <div className="form-field span-2">
                <label>Przykłady użycia (jeden na linię)</label>
                <textarea value={form.examples} onChange={e => setForm({...form, examples:e.target.value})}
                  placeholder="The ephemeral beauty of cherry blossoms..." rows={2} />
              </div>
              <div className="form-field">
                <label>Synonimy (oddzielone przecinkami)</label>
                <input value={form.synonyms} onChange={e => setForm({...form, synonyms:e.target.value})}
                  placeholder="fleeting, transient" />
              </div>
              <div className="form-field">
                <label>Antonimy (oddzielone przecinkami)</label>
                <input value={form.antonyms} onChange={e => setForm({...form, antonyms:e.target.value})}
                  placeholder="permanent, enduring" />
              </div>
              <div className="form-field">
                <label>Tagi (oddzielone przecinkami)</label>
                <input value={form.tags} onChange={e => setForm({...form, tags:e.target.value})}
                  placeholder="literackie, egzamin" />
              </div>
              <div className="form-field">
                <label>Poziom trudności:{" "}
                  <span style={{color:DIFFICULTY_COLORS[form.difficulty]}}>
                    {DIFFICULTY_LABELS[form.difficulty]}
                  </span>
                </label>
                <input type="range" min={1} max={5} step={1} value={form.difficulty}
                  onChange={e => setForm({...form, difficulty:+e.target.value})}
                  className="difficulty-slider" />
                <div className="slider-labels"><span>Łatwy</span><span>Trudny</span></div>
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

// ─── Vocab Row ─────────────────────────────────────────────────────────────────
// Accepts WordWithProgress — backwards-compatible because it's a superset of Word.

const VocabRow: React.FC<{
  word: WordWithProgress;
  onDelete: (id: number) => void;
}> = ({ word, onDelete }) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className={`vm-row ${expanded ? "expanded" : ""}`}>
      <div className="vm-row-main" onClick={() => setExpanded(!expanded)}>
        <div className="vm-row-left">
          <span className="vm-term">{word.term}</span>
          <span className="vm-pos">{PART_OF_SPEECH_LABELS[word.partOfSpeech as PartOfSpeech] ?? word.partOfSpeech}</span>
          {word.phonetic && <span className="vm-phonetic">{word.phonetic}</span>}
          {/* {word.createdAt && <span className="vm-created-at">Dodano: {new Date(word.createdAt).toLocaleDateString()}</span>} */}
        </div>
        <div className="vm-row-center">
          <span className="vm-def-preview">{word.definition}</span>
          {word.definitionPl && (
            <span className="vm-def-pl-preview">
              <span className="pl-flag">🇵🇱</span>{word.definitionPl}
            </span>
          )}
          {/* SRS Badge — the key addition */}
          <SrsBadge word={word} />
        </div>
        <div className="vm-row-right">
          <span className="vm-diff-badge" style={{
            background: `${DIFFICULTY_COLORS[word.difficulty]}20`,
            color: DIFFICULTY_COLORS[word.difficulty],
            borderColor: `${DIFFICULTY_COLORS[word.difficulty]}40`,
          }}>
            {DIFFICULTY_LABELS[word.difficulty]}
          </span>
          <button className="vm-delete" onClick={e => { e.stopPropagation(); onDelete(word.id); }}
            aria-label="Usuń">🗑</button>
          <span className="vm-expand-icon">{expanded ? "▲" : "▼"}</span>
        </div>
      </div>

      {expanded && (
        <div className="vm-row-detail">
          {/* Sentences */}
          {word.sentencePl && (
            <div className="detail-section">
              <span className="detail-label">🇵🇱 Zdanie PL</span>
              <div className="detail-sentence">{word.sentencePl}</div>
            </div>
          )}
          {word.sentenceEn && (
            <div className="detail-section">
              <span className="detail-label">🇬🇧 Zdanie EN</span>
              <div className="detail-sentence">{word.sentenceEn}</div>
            </div>
          )}
          {/* SRS detail row */}
          <div className="detail-section srs-detail-row">
            <span className="detail-label">SRS</span>
            <div className="srs-detail-grid">
              <span>Powtórzeń: <b>{word.repetitions}</b></span>
              <span>Interwał: <b>{word.intervalDays < 1
                ? `${Math.round(word.intervalDays * 24 * 60)} min`
                : `${word.intervalDays.toFixed(1)} dni`}
              </b></span>
              <span>EF: <b>{word.easeFactor.toFixed(2)}</b></span>
              <span>Passa: <b>{word.streak}</b></span>
              <span>Łącznie: <b>{word.totalReviews}</b></span>
            </div>
          </div>
          {word.tags.length > 0 && (
            <div className="chip-row">
              {word.tags.map(t => <span key={t} className="chip chip-tag">#{t}</span>)}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
