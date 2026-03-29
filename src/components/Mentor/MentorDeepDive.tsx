// src/components/Mentor/MentorDeepDive.tsx
import React from "react";
import type { Word, MentorTip } from "../../types";
import "./MentorCenter.css";

interface MentorDeepDiveProps {
  word: Word;
  tip: MentorTip;
  onBack: () => void;
  onClose: () => void;
}

export const MentorDeepDive: React.FC<MentorDeepDiveProps> = ({ word, tip, onBack, onClose }) => {
  return (
    <div className="mentor-deep-dive">
      <div className="mentor-header">
        <button className="mentor-back-btn" onClick={onBack}>← Wróć do listy</button>
        <button className="mentor-close-btn" onClick={onClose}>✕ Zamknij</button>
      </div>

      <div className="mdd-card">
        <div className="mdd-main-info">
          <h2 className="mdd-term">{word.term}</h2>
          <div className="mdd-def">{word.definition}</div>
          {word.definitionPl && (
            <div className="mdd-def-pl">
              <span className="pl-flag">🇵🇱</span> {word.definitionPl}
            </div>
          )}
        </div>

        <div className="mdd-sections">
          <div className="mdd-section">
            <h3 className="mdd-sec-title">🧠 Mnemotechnika (Haczyk pamięciowy)</h3>
            <p className="mdd-sec-content mdd-mnemonic">{tip.mnemonic}</p>
          </div>

          <div className="mdd-section">
            <h3 className="mdd-sec-title">🏢 Kontekst Biznesowy / IT</h3>
            <div className="mdd-sec-content mdd-story">
              {tip.businessStory.split("--").map((part, i) => 
                i % 2 === 1 ? <strong key={i}>{part}</strong> : part
              )}
            </div>
          </div>

          <div className="mdd-section">
            <h3 className="mdd-sec-title">🔍 Głębsze Wyjaśnienie</h3>
            <p className="mdd-sec-content">{tip.deepDiveExplanation}</p>
          </div>
        </div>

        <div className="mdd-footer">
          <p className="mdd-advice">Analiza wykonana przez AI Mentora na podstawie Twoich ostatnich sesji.</p>
        </div>
      </div>
    </div>
  );
};
