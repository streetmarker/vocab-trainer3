// src/components/Mentor/MentorCenter.tsx
import React, { useEffect, useState } from "react";
import { api } from "../../hooks/useTauri";
import type { Word, MentorTip } from "../../types";
import { MentorDeepDive } from "./MentorDeepDive";
import "./MentorCenter.css";

interface MentorCenterProps {
  onClose: () => void;
}

export const MentorCenter: React.FC<MentorCenterProps> = ({ onClose }) => {
  const [strugglingWords, setStrugglingWords] = useState<Word[]>([]);
  const [tips, setTips] = useState<Record<number, MentorTip>>({});
  const [selectedWordId, setSelectedWordId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      api.getStrugglingWords(10),
      api.getMentorTips()
    ]).then(([words, allTips]) => {
      setStrugglingWords(words);
      setTips(allTips);
      setLoading(false);
    }).catch(err => {
      console.error(err);
      setLoading(false);
    });
  }, []);

  if (loading) return <div className="mentor-loading">Ładowanie analizy mentora...</div>;

  // Filtrujemy tylko te słowa, które mają już wygenerowane wskazówki
  const wordsWithTips = strugglingWords.filter(w => !!tips[w.id]);

  if (selectedWordId && tips[selectedWordId]) {
    const word = strugglingWords.find(w => w.id === selectedWordId)!;
    return (
      <MentorDeepDive 
        word={word} 
        tip={tips[selectedWordId]} 
        onBack={() => setSelectedWordId(null)} 
        onClose={onClose}
      />
    );
  }

  return (
    <div className="mentor-center">
      <div className="mentor-header">
        <div className="mentor-title-group">
          <span className="mentor-icon">🧠</span>
          <div>
            <h1 className="mentor-title">Centrum Mentora AI</h1>
            <p className="mentor-subtitle">Analiza Twoich najczęstszych błędów i trudności</p>
          </div>
        </div>
        <button className="mentor-close-btn" onClick={onClose}>✕ Zamknij</button>
      </div>

      <div className="mentor-content">
        {wordsWithTips.length === 0 ? (
          <div className="mentor-empty">
            <p>Nie znaleziono jeszcze słów wymagających interwencji mentora.</p>
            <p className="mentor-empty-sub">Ćwicz dalej, a Mentor pojawi się, gdy zauważy Twoje trudności.</p>
          </div>
        ) : (
          <div className="mentor-list">
            <h3 className="mentor-list-title">Słowa wymagające "Głębokiej Powtórki":</h3>
            <div className="mentor-cards">
              {wordsWithTips.map(word => (
                <div key={word.id} className="mentor-word-card" onClick={() => setSelectedWordId(word.id)}>
                  <div className="mwc-term">{word.term}</div>
                  <div className="mwc-reason">Wykryto częste błędy lub długi czas reakcji</div>
                  <button className="mwc-action-btn">Rozpocznij Deep Dive →</button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
