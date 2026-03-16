// src/components/ImportWords/ImportWords.tsx
//
// Self-contained import widget. Drop it anywhere — it renders as a single
// "Importuj słówka z pliku" button that opens a hidden <input type="file">.
// After a successful import it shows a result card inline (added / pominięto).

import React, { useRef, useState } from "react";
import { api } from "../../hooks/useTauri";
import "./ImportWords.css";

type Phase = "idle" | "loading" | "done" | "error";

interface ImportResult {
  added:    number;
  skipped:  number;
  warnings: string[];
}

interface Props {
  /** Called after a successful import so the parent can refresh its word list */
  onImportDone?: () => void;
}

const SCHEMA_EXAMPLE = `[
  {
    "term": "ubiquitous",
    "definition": "Present everywhere at the same time",
    "definitionPl": "wszechobecny",
    "partOfSpeech": "adjective",
    "phonetic": "/juːˈbɪk.wɪ.təs/",
    "difficulty": 3,
    "zdaniePL": "Smartfony są ubiquitous w dzisiejszym świecie.",
    "zdanieEN": "Smartphones have become ubiquitous in modern life.",
    "examples": ["Technology is ubiquitous in modern life."],
    "synonyms": ["omnipresent", "pervasive"],
    "antonyms": ["rare", "scarce"],
    "tags": ["C1", "formal"]
  }
]`;

const REQUIRED_FIELDS = [`"term"`, `"definition"`];
const OPTIONAL_FIELDS = [
  `"definitionPl"`, `"partOfSpeech"`, `"phonetic"`,
  `"difficulty" (1–5)`,
  `"zdaniePL"`, `"zdanieEN"`,
  `"examples" []`, `"synonyms" []`, `"antonyms" []`, `"tags" []`,
];

export const ImportWords: React.FC<Props> = ({ onImportDone }) => {
  const fileRef              = useRef<HTMLInputElement>(null);
  const [phase, setPhase]    = useState<Phase>("idle");
  const [result, setResult]  = useState<ImportResult | null>(null);
  const [errMsg, setErrMsg]  = useState("");
  const [infoOpen, setInfoOpen] = useState(false);

  // ── File selected ──────────────────────────────────────────────────────────
  const handleFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // Reset input so the same file can be re-picked after fixing errors
    e.target.value = "";

    setPhase("loading");
    setResult(null);
    setErrMsg("");

    try {
      const text = await file.text();
      const result = await api.importWordsFromJson(text);
      setResult(result);
      setPhase("done");
      if (result.added > 0) onImportDone?.();
    } catch (err: unknown) {
      setErrMsg(err instanceof Error ? err.message : String(err));
      setPhase("error");
    }
  };

  const reset = () => { setPhase("idle"); setResult(null); setErrMsg(""); };

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div className="iw-root">
      {/* hidden native file input */}
      <input
        ref={fileRef}
        type="file"
        accept=".json,application/json"
        style={{ display: "none" }}
        onChange={handleFile}
      />

      {/* ── Main action row ── */}
      <div className="iw-action-row">
        <button
          className="btn-secondary iw-btn"
          disabled={phase === "loading"}
          onClick={() => fileRef.current?.click()}
        >
          {phase === "loading" ? (
            <><span className="iw-spinner" /> Importowanie…</>
          ) : (
            <>⬆ Importuj słówka z pliku</>
          )}
        </button>

        <button
          className="iw-info-toggle"
          title="Pokaż format JSON"
          onClick={() => setInfoOpen((v) => !v)}
          aria-expanded={infoOpen}
        >
          {infoOpen ? "✕" : "?"}
        </button>
      </div>

      {/* ── Info box ── */}
      {infoOpen && (
        <div className="iw-info">
          <div className="iw-info-title">Format pliku JSON</div>
          <p className="iw-info-text">
            Plik musi zawierać tablicę obiektów. Wymagane pola:{" "}
            {REQUIRED_FIELDS.map((f, i) => (
              <span key={i} className="iw-badge iw-badge--req">{f}</span>
            ))}
          </p>
          <p className="iw-info-text">
            Opcjonalne:{" "}
            {OPTIONAL_FIELDS.map((f, i) => (
              <span key={i} className="iw-badge">{f}</span>
            ))}
          </p>
          <pre className="iw-code">{SCHEMA_EXAMPLE}</pre>
          <p className="iw-info-note">
            Słówka z istniejącym <code>term</code> są pomijane (duplikaty).
          </p>
        </div>
      )}

      {/* ── Result card ── */}
      {phase === "done" && result && (
        <div className="iw-result iw-result--ok">
          <div className="iw-result-row">
            <span className="iw-stat iw-stat--added">
              ✓ {result.added} dodano
            </span>
            {result.skipped > 0 && (
              <span className="iw-stat iw-stat--skipped">
                ⊘ {result.skipped} pominięto
              </span>
            )}
            <button className="iw-dismiss" onClick={reset}>✕</button>
          </div>

          {result.warnings.length > 0 && (
            <details className="iw-warnings">
              <summary>
                {result.warnings.length} ostrzeżenie
                {result.warnings.length !== 1 ? "ń" : ""}
              </summary>
              <ul>
                {result.warnings.map((w, i) => <li key={i}>{w}</li>)}
              </ul>
            </details>
          )}
        </div>
      )}

      {/* ── Error card ── */}
      {phase === "error" && (
        <div className="iw-result iw-result--err">
          <span className="iw-err-icon">⚠</span>
          <span className="iw-err-msg">{errMsg}</span>
          <button className="iw-dismiss" onClick={reset}>✕</button>
        </div>
      )}
    </div>
  );
};
