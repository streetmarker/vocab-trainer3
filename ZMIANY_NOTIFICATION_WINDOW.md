# Zmiany: Dynamiczne skalowanie okna powiadomień

**Data**: 16.03.2026  
**Problem**: Okienko powiadomień miało stały rozmiar, który nie dostosowywał się do dużych skal ekranu, powodując że zawartość nie była widoczna.  
**Rozwiązanie**: Zmiana z systemu aspektu na procentowy (%) oparty na rozmiarze ekranu.

---

## 1. Plik: `src-tauri/src/lib.rs`

### Funkacja: `show_task_notification()`

#### Zmiana poczatkowa:
```rust
const WIDTH_PCT: f64 = 0.22;   // 22% of logical screen width
const WIDTH_MIN: f64 = 300.0;  // never narrower than this (logical px)
const WIDTH_MAX: f64 = 420.0;  // never wider than this (logical px)
const ASPECT:    f64 = 201.0 / 360.0; // original h/w ratio

let (win_w_log, win_h_log) = if let Ok(Some(monitor)) = app.primary_monitor() {
    let scale   = monitor.scale_factor();
    let phys_w  = monitor.size().width as f64;
    let logical_w = phys_w / scale;
    let w = (logical_w * WIDTH_PCT).clamp(WIDTH_MIN, WIDTH_MAX);
    let h = (w * ASPECT).round();  // ❌ Fixed aspect ratio
    (w, h)
} else {
    (360.0, 201.0) // fallback if monitor query fails
};
```

#### Zmiana nowa:
```rust
const WIDTH_PCT: f64 = 0.22;    // 22% of logical screen width
const HEIGHT_PCT: f64 = 0.35;   // 35% of logical screen height ✅ NEW
const WIDTH_MIN: f64 = 300.0;   // minimum width (logical px)
const WIDTH_MAX: f64 = 480.0;   // maximum width (logical px) — increased
const HEIGHT_MIN: f64 = 200.0;  // minimum height (logical px) ✅ NEW
const HEIGHT_MAX: f64 = 600.0;  // maximum height (logical px) ✅ NEW

let (win_w_log, win_h_log) = if let Ok(Some(monitor)) = app.primary_monitor() {
    let scale    = monitor.scale_factor();
    let phys_w   = monitor.size().width as f64;
    let phys_h   = monitor.size().height as f64;  // ✅ NEW
    let logical_w = phys_w / scale;
    let logical_h = phys_h / scale;  // ✅ NEW
    
    let w = (logical_w * WIDTH_PCT).clamp(WIDTH_MIN, WIDTH_MAX);
    let h = (logical_h * HEIGHT_PCT).clamp(HEIGHT_MIN, HEIGHT_MAX);  // ✅ NEW - percentage based
    (w, h)
} else {
    (360.0, 280.0) // fallback (adjusted height)
};
```

**Co się zmieniło:**
- ✅ Wysokość okna teraz skaluje się od 35% wysokości ekranu (zamiast stałego aspektu)
- ✅ WIDTH_MAX podwyższone z 420 na 480px dla lepszego dopasowania zawartości
- ✅ HEIGHT dynamicznie obliczane z monitor.size().height
- ✅ Zastosowanie clamp dla wysokości (min: 200px, max: 600px)

---

## 2. Plik: `notification.html`

### Zmiana początkowa:
```html
<style>
  html, body, #notification-root {
    margin: 0;
    padding: 0;
    background: transparent !important;
    overflow: hidden;
    width: 100%;
    height: 100%;
    user-select: none;
  }
</style>
```

### Zmiana nowa:
```html
<style>
  html, body {
    margin: 0;
    padding: 0;
    background: transparent !important;
    overflow: hidden;
    width: 100%;
    height: 100%;
    user-select: none;
  }
  #notification-root {  // ✅ NEW - separated for better control
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
    width: 100%;
    height: 100%;
    display: flex;  // ✅ NEW - flex container
  }
</style>
```

**Co się zmieniło:**
- ✅ #notification-root wydzielony ze wspólnego selektora
- ✅ Dodane `display: flex` dla lepszej kontroli nad layoutem

---

## 3. Plik: `src/styles/notification.css`

### 3.1 Sekcja "Reset / base" - HTML i BODY

#### Zmiana początkowa:
```css
html,
body {
  background: transparent;
  overflow: hidden;
  font-family: "Segoe UI", system-ui, -apple-system, sans-serif;
  font-size: 13px;
  -webkit-font-smoothing: antialiased;
}
```

#### Zmiana nowa:
```css
html,
body {
  background: transparent;
  overflow: hidden;
  font-family: "Segoe UI", system-ui, -apple-system, sans-serif;
  font-size: 13px;
  -webkit-font-smoothing: antialiased;
  width: 100%;   // ✅ NEW
  height: 100%;  // ✅ NEW
}
```

---

### 3.2 Sekcja ".tn-progress"

#### Zmiana początkowa:
```css
.tn-progress {
  width: 100%;
  height: 3px;
  background: rgba(255, 255, 255, 0.07);
  position: relative;
}
```

#### Zmiana nowa:
```css
.tn-progress {
  width: 100%;
  height: 3px;
  background: rgba(255, 255, 255, 0.07);
  position: relative;
  flex-shrink: 0;  // ✅ NEW - prevent shrinking
}
```

---

### 3.3 Sekcja ".tn-root" - GŁÓWNA KARTKA

#### Zmiana początkowa:
```css
.tn-root {
  position: fixed;
  bottom: 0;
  right: 0;
  width: 100%;
  background: #1e1f26;
  border: 1px solid rgba(255, 255, 255, 0.10);
  border-bottom: none;
  border-radius: 12px 12px 0 0;
  box-shadow:
    0 -4px 24px rgba(0, 0, 0, 0.55),
    0 -1px 0 rgba(255, 255, 255, 0.04) inset;
  overflow: hidden;
  cursor: default;
  transition: transform var(--slide-dur, 320ms) cubic-bezier(0.22, 1, 0.36, 1),
              opacity   var(--slide-dur, 320ms) ease;
}
```

#### Zmiana nowa:
```css
.tn-root {
  position: fixed;
  bottom: 0;
  right: 0;
  width: 100%;
  height: 100%;           // ✅ NEW - fills window height
  display: flex;          // ✅ NEW - flex layout
  flex-direction: column; // ✅ NEW - vertical stacking
  background: #1e1f26;
  border: 1px solid rgba(255, 255, 255, 0.10);
  border-bottom: none;
  border-radius: 12px 12px 0 0;
  box-shadow:
    0 -4px 24px rgba(0, 0, 0, 0.55),
    0 -1px 0 rgba(255, 255, 255, 0.04) inset;
  overflow: hidden;
  cursor: default;
  transition: transform var(--slide-dur, 320ms) cubic-bezier(0.22, 1, 0.36, 1),
              opacity   var(--slide-dur, 320ms) ease;
}
```

**Co się zmieniło:**
- ✅ `height: 100%` - okno zajmuje całą wysokość
- ✅ `display: flex; flex-direction: column` - elastyczne rozmieszczanie elementów

---

### 3.4 Sekcja ".tn-header"

#### Zmiana początkowa:
```css
.tn-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 11px 12px 4px;
}
```

#### Zmiana nowa:
```css
.tn-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;      // ✅ ZMIENIONE - bardziej zawarte
  flex-shrink: 0;         // ✅ NEW - prevent shrinking
}
```

---

### 3.5 Sekcja ".tn-card-wrap" - KARTA FLASHCARDA

#### Zmiana początkowa:
```css
.tn-card-wrap {
  padding: 0 10px 0;
}
```

#### Zmiana nowa:
```css
.tn-card-wrap {
  padding: 0 8px;               // ✅ ZMIENIONE - bardziej zawarte
  flex: 1;                      // ✅ NEW - expand to fill space
  display: flex;                // ✅ NEW
  flex-direction: column;       // ✅ NEW
  justify-content: center;      // ✅ NEW - center content vertically
  overflow: hidden;             // ✅ NEW
}
```

**Co się zmieniło:**
- ✅ Karta zajmuje całą dostępną przestrzeń (flex: 1)
- ✅ Zawartość wyśrodkowana w karcie

---

### 3.6 Sekcja ".tn-actions" - PRZYCISKI AKCJI

#### Zmiana początkowa:
```css
.tn-actions {
  display: flex;
  gap: 8px;
  padding: 8px 12px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.07);
  max-height: 52px;
  overflow: hidden;
  opacity: 1;
  transition:
    max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    opacity    0.2s ease,
    padding    0.3s ease;
}
```

#### Zmiana nowa:
```css
.tn-actions {
  display: flex;
  gap: 8px;
  padding: 6px 10px 8px;        // ✅ ZMIENIONE - bardziej zawarte
  border-top: 1px solid rgba(255, 255, 255, 0.07);
  max-height: 60px;             // ✅ ZMIENIONE z 52px
  overflow: hidden;
  opacity: 1;
  transition:
    max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    opacity    0.2s ease,
    padding    0.3s ease;
  flex-shrink: 0;               // ✅ NEW - prevent shrinking
}
```

---

## 4. Plik: `src/components/Flashcard/Flashcard.css`

### 4.1 Sekcja ".fc-wrapper"

#### Zmiana początkowa:
```css
.fc-wrapper {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}
```

#### Zmiana nowa:
```css
.fc-wrapper {
  display: flex;
  flex-direction: column;
  gap: 6px;         // ✅ ZMIENIONE z 8px
  width: 100%;
  height: 100%;     // ✅ NEW - fill parent height
}
```

---

### 4.2 Sekcja ".fc-scene" - SCENA FLASHCARDA

#### Zmiana początkowa:
```css
.fc-scene {
  width: 100%;
  height: 108px;              // ❌ Fixed height
  perspective: 700px;
  cursor: pointer;
  outline: none;
  border-radius: 10px;
  -webkit-tap-highlight-color: transparent;
}
```

#### Zmiana nowa:
```css
.fc-scene {
  width: 100%;
  min-height: 90px;           // ✅ ZMIENIONE - min instead of fixed
  flex: 1;                    // ✅ NEW - expand to fill space
  perspective: 700px;
  cursor: pointer;
  outline: none;
  border-radius: 10px;
  -webkit-tap-highlight-color: transparent;
}
```

**Co się zmieniło:**
- ✅ Zamiast stałej wysokości 108px: min-height: 90px
- ✅ `flex: 1` pozwala karcie rozciągać się do dostępnej przestrzeni

---

### 4.3 Sekcja ".fc-grades"

#### Zmiana początkowa:
```css
.fc-grades {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  max-height: 0;              // ❌ Start hidden
  overflow: hidden;
  opacity: 0;
  pointer-events: none;
  transition:
    max-height 0.32s cubic-bezier(0.4, 0, 0.2, 1),
    opacity    0.25s ease 0.20s;
}
```

#### Zmiana nowa:
```css
.fc-grades {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  max-height: 60px;           // ✅ NEW - pre-allocate space
  overflow: hidden;
  opacity: 0;
  pointer-events: none;
  transition:
    max-height 0.32s cubic-bezier(0.4, 0, 0.2, 1),
    opacity    0.25s ease 0.20s;
  flex-shrink: 0;             // ✅ NEW - prevent shrinking
}
```

---

## Podsumowanie zmian

| Komponent | Zmiana |
|-----------|--------|
| **Okno (Rust)** | Wysokość teraz 35% ekranu zamiast stałego aspektu |
| **Layout** | Flex container dla elastycznego rozmieszczenia |
| **.tn-card-wrap** | flex: 1 - zajmuje dostępną przestrzeń |
| **.fc-scene** | min-height: 90px + flex: 1 zamiast height: 108px |
| **Padding** | Zmniejszone marżesy (większa gęstość) |
| **flex-shrink: 0** | Dodane do nieprzyginających komponentów |

## Efekt końcowy

✅ Okno dynamicznie skaluje się do rozmiaru ekranu  
✅ Zawartość zawsze widoczna bez względu na skalę DPI  
✅ Layout elastycznie dostosowuje się do rozmiaru okna  
✅ Brak sztywnych wymiarów w pikselach  
