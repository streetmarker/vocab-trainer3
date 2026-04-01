// src/App.tsx
import React, { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { Dashboard } from "./components/Dashboard/Dashboard";
import { VocabManager } from "./components/VocabManager/VocabManager";
import type { AppRoute } from "./types";
import { api } from "./hooks/useTauri";
import "./styles/global.css";


export default function App() {
  const [route, setRoute] = useState<AppRoute>("dashboard");
  const [activeCategory, setActiveCategory] = useState<string>("Wszystkie");
  
  useEffect(() => {
    // Priorytet: Ustawienia z backendu (settings.json)
    // Fallback: localStorage
    api.getSettings().then(s => {
      if (s.activeCategory) {
        setActiveCategory(s.activeCategory);
        localStorage.setItem("active_category", s.activeCategory);
      } else {
        const saved = localStorage.getItem("active_category");
        if (saved) {
          setActiveCategory(saved);
          api.setActiveCategory(saved === "Wszystkie" ? null : saved).catch(console.error);
        }
      }
    }).catch(() => {
      const saved = localStorage.getItem("active_category");
      if (saved) setActiveCategory(saved);
    });
  }, []);

  const handleCategoryChange = (cat: string) => {
    setActiveCategory(cat);
    localStorage.setItem("active_category", cat);
    api.setActiveCategory(cat === "Wszystkie" ? null : cat).catch(console.error);
  };

  useEffect(() => {
    // Wywołujemy inicjalizację autostartu raz przy starcie aplikacji
    import('@tauri-apps/api/core').then(({ invoke }) => {
      invoke('initialize_autostart').catch(console.error);
    });
  }, []);
  // Listen for navigation events from tray menu
  useEffect(() => {
    const unlisten = listen<AppRoute>("navigate", (e) => {
      setRoute(e.payload);
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  const navItems: { id: AppRoute; icon: string; label: string }[] = [
    { id: "dashboard", icon: "📊", label: "Panel główny" },
    { id: "vocab",     icon: "📚", label: "Słownictwo" },
    { id: "settings",  icon: "⚙️", label: "Ustawienia" },
  ];

  return (
    <div className="app-root">
      {/* ── Sidebar ───────────────────────────────────────────────────── */}
      <nav className="sidebar">
        <div className="sidebar-logo">
          <div className="logo-mark">V</div>
          <div>
            <div className="logo-name">VocabTrainer</div>
            <div className="logo-tagline">nauka angielskiego</div>
          </div>
        </div>

        <div className="nav-links">
          {navItems.map((item) => (
            <button
              key={item.id}
              className={`nav-link ${route === item.id ? "active" : ""}`}
              onClick={() => setRoute(item.id)}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
              {route === item.id && <div className="nav-active-bar" />}
            </button>
          ))}
        </div>

        <CategorySelector active={activeCategory} onChange={handleCategoryChange} />

        <SchedulerStatus />
      </nav>

      {/* ── Main Content ──────────────────────────────────────────────── */}
      <main className="main-content">
        {route === "dashboard" && <Dashboard activeCategory={activeCategory} />}
        {route === "vocab" && <VocabManager activeCategory={activeCategory} />}
        {route === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}

// ─── Category Selector ────────────────────────────────────────────────────────

const CategorySelector: React.FC<{ active: string; onChange: (c: string) => void }> = ({ active, onChange }) => {
  const [categories, setCategories] = useState<string[]>(["Wszystkie"]);

  const refresh = useCallback(() => {
    // Pobierz unikalne kategorie z bazy
    api.getSrsOverview().then(data => {
      const cats = new Set<string>();
      cats.add("Wszystkie");
      data.words.forEach(w => {
        if (w.category) cats.add(w.category);
      });
      setCategories(Array.from(cats).sort());
    }).catch(console.error);
  }, []);

  useEffect(() => {
    refresh();
    window.addEventListener("refresh-categories", refresh);
    return () => window.removeEventListener("refresh-categories", refresh);
  }, [refresh]);

  return (
    <div className="category-sidebar-section">
      <div className="cat-section-label">Źródło danych</div>
      <div className="cat-list">
        {categories.map(cat => (
          <button 
            key={cat} 
            className={`cat-btn ${active === cat ? "active" : ""}`}
            onClick={() => onChange(cat)}
          >
            <span className="cat-dot" style={{ background: active === cat ? "var(--accent)" : "transparent" }} />
            {cat}
          </button>
        ))}
      </div>
    </div>
  );
};

// ─── Scheduler Status Widget ──────────────────────────────────────────────────

const SchedulerStatus: React.FC = () => {
  const [status, setStatus] = useState<any>(null);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    const poll = async () => {
      try {
        const s = await api.getSchedulerStatus();
        setStatus(s);
        setPaused(!s.conditions.notPaused);
      } catch {}
    };
    poll();
    const id = setInterval(poll, 15000);
    return () => clearInterval(id);
  }, []);

  const togglePause = async () => {
    const newPaused = !paused;
    setPaused(newPaused);
    await api.setSchedulerPaused(newPaused);
  };

  return (
    <div className="scheduler-widget">
      <div className="sched-header">
        <span className="sched-label">Harmonogram</span>
        <div className={`sched-dot ${status?.isReady ? "ready" : "waiting"}`} />
      </div>
      {status?.blockedReason && (
        <div className="sched-reason">{status.blockedReason}</div>
      )}
      <button
        className={`sched-toggle ${paused ? "paused" : ""}`}
        onClick={togglePause}
      >
        {paused ? "▶ Wznów" : "⏸ Wstrzymaj"}
      </button>
    </div>
  );
};

// ─── Settings Page ────────────────────────────────────────────────────────────

type Settings = {
  exercisesPerDay: number;
  idleThresholdSecs: number;
  minGapMinutes: number;
  autostart: boolean;
  showSessionWord: boolean;
  soundEffects: boolean;
  workHoursOnly: boolean;
  workHoursStart: string;
  workHoursEnd: string;
};

const DEFAULT_SETTINGS: Settings = {
  exercisesPerDay: 50,
  idleThresholdSecs: 5,
  minGapMinutes: 30,
  autostart: true,
  showSessionWord: true,
  soundEffects: false,
  workHoursOnly: true,
  workHoursStart: "08:00",
  workHoursEnd: "22:00",
};

const SettingsPage: React.FC = () => {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [selectedDate, setSelectedDate] = useState<string>("");

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setLoaded(true);
    }).catch(() => setLoaded(true));
  }, []);

  const handleDeleteByDate = async () => {
    if (!selectedDate) {
      alert("Proszę wybrać datę.");
      return;
    }

    const confirmed = confirm(
      `Czy na pewno chcesz usunąć wszystkie fiszki (wraz ze statystykami) z dnia ${selectedDate}?\n\nOperacji nie można cofnąć.`
    );

    if (!confirmed) return;

    setClearing(true);
    try {
      const deleted = await api.deleteWordsByBatchDate(selectedDate);
      alert(`Pomyślnie usunięto ${deleted} fiszek.`);
      window.dispatchEvent(new CustomEvent("refresh-categories"));
    } catch (err) {
      alert("Błąd podczas usuwania: " + err);
    } finally {
      setClearing(false);
    }
  };

  const handleClearWords = async () => {
    const confirmed = confirm(
      "Czy na pewno chcesz wyczyścić całą bazę słówek?\n\n" +
      "Zostaną usunięte wszystkie słowa oraz cały postęp nauki (SRS, powtórki, historia).\n\n" +
      "Tej operacji nie można cofnąć."
    );
    if (!confirmed) return;
    setClearing(true);
    try {
      const count = await api.clearWords();
      alert(`Usunięto ${count} słów i cały powiązany postęp nauki.`);
    } catch (e: any) {
      alert("Błąd podczas czyszczenia bazy: " + e.toString());
    } finally {
      setClearing(false);
    }
  };

  const update = async (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    setSaving(true);
    try {
      await api.saveSettings(next);
    } finally {
      setSaving(false);
    }
  };

  if (!loaded) return (
    <div className="settings-page">
      <h1 className="settings-title">Ustawienia</h1>
      <div style={{ color: "var(--muted)", padding: "32px 0" }}>Ładowanie…</div>
    </div>
  );

  return (
    <div className="settings-page">
      <h1 className="settings-title">
        Ustawienia
        {saving && <span style={{ fontSize: 12, color: "var(--muted)", marginLeft: 12, fontWeight: 400 }}>Zapisywanie…</span>}
      </h1>
      <div className="settings-sections">

        <SettingsSection title="Harmonogram">
          <SettingRow
            label="Ćwiczenia dziennie"
            description="Maksymalna liczba wyskakujących okienek dziennie"
            control={
              <input
                type="number"
                value={settings.exercisesPerDay}
                min={5} max={200}
                className="setting-input"
                onChange={(e) => update({ exercisesPerDay: Number(e.target.value) })}
              />
            }
          />
          <SettingRow
            label="Próg bezczynności"
            description="Sekundy bezczynności przed pokazaniem ćwiczenia"
            control={
              <input
                type="number"
                value={settings.idleThresholdSecs}
                min={1} max={60}
                className="setting-input"
                onChange={(e) => update({ idleThresholdSecs: Number(e.target.value) })}
              />
            }
          />
          <SettingRow
            label="Minimalny odstęp między ćwiczeniami"
            description="Minuty przerwy między wyskakującymi okienkami"
            control={
              <input
                type="number"
                value={settings.minGapMinutes}
                min={5} max={120}
                className="setting-input"
                onChange={(e) => update({ minGapMinutes: Number(e.target.value) })}
              />
            }
          />
        </SettingsSection>

        <SettingsSection title="Nauka">
          <SettingRow
            label="Uruchamiaj z Windowsem"
            description="Uruchom VocabTrainer automatycznie po zalogowaniu"
            control={<Toggle on={settings.autostart} onChange={(v) => update({ autostart: v })} />}
          />
          <SettingRow
            label="Pokaż słowo sesji"
            description="Wyświetl kartę wprowadzającą przy starcie komputera"
            control={<Toggle on={settings.showSessionWord} onChange={(v) => update({ showSessionWord: v })} />}
          />
          <SettingRow
            label="Efekty dźwiękowe"
            description="Odtwarzaj dźwięki przy poprawnych/błędnych odpowiedziach"
            control={<Toggle on={settings.soundEffects} onChange={(v) => update({ soundEffects: v })} />}
          />
        </SettingsSection>

        <SettingsSection title="Godziny pracy">
          <SettingRow
            label="Tylko w godzinach pracy"
            description="Ogranicz ćwiczenia do określonych godzin"
            control={<Toggle on={settings.workHoursOnly} onChange={(v) => update({ workHoursOnly: v })} />}
          />
          <SettingRow
            label="Godziny pracy"
            description="Czas rozpoczęcia i zakończenia dostarczania ćwiczeń"
            control={
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input
                  type="time"
                  value={settings.workHoursStart}
                  className="setting-input"
                  onChange={(e) => update({ workHoursStart: e.target.value })}
                />
                <span style={{ color: "var(--muted)" }}>do</span>
                <input
                  type="time"
                  value={settings.workHoursEnd}
                  className="setting-input"
                  onChange={(e) => update({ workHoursEnd: e.target.value })}
                />
              </div>
            }
          />
        </SettingsSection>

        <SettingsSection title="Zarządzanie danymi">
          <SettingRow
            label="Usuń fiszki z dnia"
            description="Usuwa wszystkie fiszki dodane w konkretnym dniu wraz z ich historią nauki."
            control={
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input
                  type="date"
                  value={selectedDate}
                  className="setting-input"
                  onChange={(e) => setSelectedDate(e.target.value)}
                />
                <button
                  className="btn-danger"
                  onClick={handleDeleteByDate}
                  disabled={clearing || !selectedDate}
                  style={{ whiteSpace: "nowrap" }}
                >
                  {clearing ? "Usuwanie…" : "🗑 Usuń z tego dnia"}
                </button>
              </div>
            }
          />
        </SettingsSection>

        <SettingsSection title="Niebezpieczna strefa">
          <SettingRow
            label="Wyczyść całą bazę słówek"
            description="Trwale usuwa WSZYSTKIE słowa i cały postęp nauki. Operacji nie można cofnąć."
            control={
              <button
                className="btn-danger"
                onClick={handleClearWords}
                disabled={clearing}
              >
                {clearing ? "Usuwanie…" : "💀 Wyczyść wszystko"}
              </button>
            }
          />
        </SettingsSection>

      </div>
    </div>
  );
};

const SettingsSection: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <div className="settings-section">
    <h2 className="settings-section-title">{title}</h2>
    <div className="settings-rows">{children}</div>
  </div>
);

const SettingRow: React.FC<{ label: string; description: string; control: React.ReactNode }> = ({
  label, description, control,
}) => (
  <div className="setting-row">
    <div className="setting-info">
      <div className="setting-label">{label}</div>
      <div className="setting-desc">{description}</div>
    </div>
    <div className="setting-control">{control}</div>
  </div>
);

const Toggle: React.FC<{ on: boolean; onChange: (v: boolean) => void }> = ({ on, onChange }) => (
  <button
    className={`toggle ${on ? "on" : ""}`}
    onClick={() => onChange(!on)}
    role="switch"
    aria-checked={on}
  >
    <div className="toggle-thumb" />
  </button>
);
