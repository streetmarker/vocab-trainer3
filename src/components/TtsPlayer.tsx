import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface TtsPlayerProps {
  term: string;
  exampleEn: string;
  autoPlay?: boolean;
}

const TtsPlayer: React.FC<TtsPlayerProps> = ({ term, exampleEn, autoPlay }) => {
  const [isPlaying, setIsPlaying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const hasAutoPlayedRef = React.useRef(false);

  const handlePlay = React.useCallback(async (e?: React.MouseEvent) => {
    if (e) e.stopPropagation(); 
    try {
      setError(null);
      setIsPlaying(true);
      
      // Wywołanie backendu Rust
      const audioPath = await invoke<string>('play_or_generate_tts', {
        payload: {
          term,
          text: `${term}. ${exampleEn}`
        }
      });

      // Odtworzenie dźwięku (HTML5 Audio)
      const audio = new Audio(audioPath);
      audio.onended = () => setIsPlaying(false);
      await audio.play();
      
    } catch (err) {
      console.error('TTS Error:', err);
      setError(String(err));
      setIsPlaying(false);
    }
  }, [term, exampleEn]);

  React.useEffect(() => {
    if (autoPlay && !hasAutoPlayedRef.current) {
      hasAutoPlayedRef.current = true;
      handlePlay();
    }
  }, [autoPlay, handlePlay]);

  return (
    <div className="tts-container" style={{ marginTop: '8px', display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }} onClick={handlePlay}>
      <span role="img" aria-label="speaker" style={{ fontSize: '1.2rem', opacity: isPlaying ? 0.5 : 1 }}>
        🔊
      </span>
      <span style={{ fontSize: '0.9rem', color: 'var(--muted)', textDecoration: 'underline' }}>
        {isPlaying ? 'Odtwarzanie...' : 'Posłuchaj zdania'}
      </span>
      {error && <small style={{ color: 'red', marginLeft: '4px' }}>{error}</small>}
    </div>
  );
};

export default TtsPlayer;
