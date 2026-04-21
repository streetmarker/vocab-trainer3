// src/components/ExercisePopup/StackedExercise.tsx

import React, { useState, useEffect, useMemo } from "react";
import { api } from "../../hooks/useTauri";
import { Word, PART_OF_SPEECH_LABELS } from "../../types";
import TtsPlayer from "../TtsPlayer";
import "./StackedExercise.css";

interface StackedExerciseProps {
  activeCategory: string;
  onClose: () => void;
}

export const StackedExercise: React.FC<StackedExerciseProps> = ({ activeCategory, onClose }) => {
  const [words, setWords] = useState<Word[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeWordId, setActiveWordId] = useState<number | null>(null);
  const [visitedIds, setVisitedIds] = useState<Set<number>>(new Set());
  const [isFlipped, setIsFlipped] = useState(false);

  useEffect(() => {
    api.getWords().then(allWords => {
      const filtered = allWords.filter(w => 
        activeCategory === "Wszystkie" || w.category === activeCategory
      );
      setWords(filtered.reverse());
      setLoading(false);
    }).catch(err => {
      console.error("Failed to fetch words:", err);
      setLoading(false);
    });
  }, [activeCategory]);

  const activeWord = useMemo(() => words.find(w => w.id === activeWordId), [words, activeWordId]);

  const handleCardClick = (id: number) => {
    setActiveWordId(id);
    setIsFlipped(false);
  };

  const handleCloseCard = () => {
    if (activeWordId !== null) {
      setVisitedIds(prev => new Set(prev).add(activeWordId));
    }
    setActiveWordId(null);
    setIsFlipped(false);
  };

  if (loading) {
    return (
      <div className="stacked-exercise-overlay">
        <div className="spinner" />
      </div>
    );
  }

  return (
    <div className="stacked-exercise-overlay">
      <div className="stacked-header">
        <h2 className="stacked-title">
          Szuflada: {activeCategory} ({visitedIds.size}/{words.length})
        </h2>
        <button className="stacked-close-btn" onClick={onClose} title="Zamknij sesję">✕</button>
      </div>

      {/* ── The Drawer (Scrollable list of tabs) ── */}
      <div className="card-drawer">
        {words.map((word, index) => {
          const isVisited = visitedIds.has(word.id);
          return (
            <div 
              key={word.id}
              className={`stacked-card-tab ${!isVisited ? 'unvisited' : ''}`}
              style={{ zIndex:  index }}
              onClick={() => handleCardClick(word.id)}
            >
              <span className="tab-label">
                {word.definitionPl || word.definition.slice(0, 30)}
              </span>
            </div>
          );
        })}
        {words.length === 0 && <div style={{ color: '#888', marginTop: 100 }}>Brak słówek</div>}
      </div>

      {/* ── Active Card Modal (Centered Overlay) ── */}
      {activeWord && (
        <div className="active-card-overlay" onClick={handleCloseCard}>
          <div className="active-card-body" onClick={e => e.stopPropagation()}>
            <button className="active-card-close-btn" onClick={handleCloseCard}>✕</button>
            
            <div className="ac-main-content">
              <div>
                <div className="ac-term-pl">{activeWord.definitionPl || "Brak tłumaczenia"}</div>
                <div className="ac-pos">{PART_OF_SPEECH_LABELS[activeWord.partOfSpeech] || activeWord.partOfSpeech}</div>
              </div>

              <div className="ac-flip-container">
                <div 
                  className={`ac-flipper ${isFlipped ? 'flipped' : ''}`}
                  onClick={() => setIsFlipped(!isFlipped)}
                >
                  <div className="ac-face ac-face--front">
                    <span className="ac-hint-text">Kliknij, aby odkryć</span>
                  </div>
                  <div className="ac-face ac-face--back">
                    <div className="ac-term-en-wrapper">
                      <div className="ac-term-en">{activeWord.term}</div>
                      <div className="ac-tts-icon-wrapper" onClick={(e) => e.stopPropagation()}>
                      </div>
                        <TtsPlayer 
                          term={activeWord.term} 
                          exampleEn={activeWord.sentenceEn || ""} 
                          autoPlay={false} 
                        />
                    </div>
                    {activeWord.phonetic && <div className="ac-phonetic">/{activeWord.phonetic}/</div>}
                    {activeWord.sentenceEn && (
                      <div className="ac-sentence-en">
                        <span className="ac-sentence-icon">💬</span>
                        <em>{activeWord.sentenceEn.replace(/--(.*?)--/g, '$1')}</em>
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <div className="ac-instructions">
                {isFlipped ? "Kliknij ponownie, by zakryć" : "Kliknij w kartę, by obrócić"}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
