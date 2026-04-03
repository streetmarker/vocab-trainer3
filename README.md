# 🇵🇱 [Polski](README.md) | 🇺🇸 [English](README_EN.md)

# 🧠 Vocab Trainer v3

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19.0-61dafb)](https://react.dev/)

**Vocab Trainer v3** to zaawansowana aplikacja desktopowa do nauki słownictwa, oparta na naukowo udowodnionej metodzie **Spaced Repetition System (SRS)**. Rozwiązuje problem braku systematyczności i trudności w organizacji fizycznych fiszek, automatyzując proces powtórek i przypominając o nauce w optymalnych momentach.

---

## 🚀 Rozwiązanie Problemu

Tradycyjna nauka z fizycznymi fiszkami wymaga ogromnej dyscypliny i skomplikowanej logiki pudełek (np. metoda Leitnera). Większość osób rezygnuje z nauki, ponieważ:
- **Brak organizacji:** Trudno śledzić, które słowo wymaga powtórki dzisiaj, a które za tydzień.
- **Brak mobilizacji:** Fizyczne fiszki nie "pukają" do nas, gdy o nich zapominamy.
- **Nieregularność:** Życie codzienne utrudnia zachowanie stałego rytmu nauki.

**Vocab Trainer v3** eliminuje te bariery, działając jako Twój osobisty mentor językowy ukryty w zasobniku systemowym. Aplikacja sama dba o harmonogram, wyświetlając dyskretne powiadomienia i popupy dokładnie wtedy, gdy Twój ślad pamięciowy zaczyna słabnąć.

---

## 📸 Prezentacja Aplikacji

### 1. Dashboard Główny
Centrum dowodzenia Twoją nauką. Tutaj widzisz statystyki, postępy oraz listę nadchodzących powtórek.
> ![Dashboard Screenshot](/panel.jpg)

### 2. Inteligentny Popup Powtórek
Kluczowa funkcja aplikacji. Pojawia się automatycznie, prezentując słowo do powtórki. Dzięki integracji z systemem, nie musisz pamiętać o uruchamianiu aplikacji.
> ![Popup Front Screenshot](/fiszka-front.jpg)
> ![Popup Back Screenshot](/fiszka-back.jpg)

---

## 🛠 Struktura Techniczna

Aplikacja została zbudowana w nowoczesnej architekturze **Tauri**, łączącej wydajność języka Rust z elastycznością Reacta.

### Stack Technologiczny:
- **Frontend:** React 19 + TypeScript + Vite
- **Desktop Core:** Tauri v2 (Rust-based security & performance)
- **Audio Service:** API Proxy do **Google TTS (Text-to-Speech)** hostowany na **Google Cloud** (zapewnia naturalną wymowę słówek)
- **Styling:** Vanilla CSS (Custom Optimized Styles)
- **Algorithm:** Zmodyfikowany **SM-2 (SuperMemo-2)** dla inteligentnego wyliczania interwałów
- **Storage:** Local JSON System (deterministryczna obsługa bazy fiszek)

---

## 🤖 AI-Driven Development

Aplikacja nie jest efektem tradycyjnego procesu kodowania, lecz zaawansowanej inżynierii promptów i współpracy z modelami LLM.

- **Framework:** Całość procesu deweloperskiego oparto na frameworku **AI Fluency**, co pozwoliło na zachowanie najwyższej jakości kodu i spójności architektonicznej.
- **Narzędzia:** Projekt został zrealizowany przy użyciu **Claude 3.5 Sonnet** (logika biznesowa i UI) oraz **Gemini CLI** (automatyzacja dokumentacji, refaktoryzacja i analiza codebase).
- **Metodologia:** Wykorzystanie "Systematic Prompting" do generowania deterministycznych komponentów oraz bezpiecznej komunikacji z API systemowym Tauri.

### Kluczowe Funkcjonalności:
- **Autostart:** Aplikacja uruchamia się wraz z systemem, działając w tle.
- **Multi-window:** Oddzielne moduły dla Dashboardu, Popupów i Notyfikacji.
- **Offline First:** Pełna funkcjonalność bez dostępu do internetu.
- **Smart Queue:** System Cooldown (12h) zapobiegający "przeuczeniu" tych samych słów w krótkich sesjach.

---

## 🏗 Instalacja i Uruchomienie

Aby uruchomić projekt lokalnie, upewnij się, że masz zainstalowane środowisko [Tauri](https://tauri.app/v1/guides/getting-started/prerequisites).

1. **Klonowanie repozytorium:**
   ```bash
   git clone https://github.com/twoje-repo/vocab-trainer3.git
   cd vocab-trainer3
   ```

2. **Instalacja zależności:**
   ```bash
   npm install
   ```

3. **Uruchomienie w trybie deweloperskim:**
   ```bash
   npm run tauri dev
   ```

4. **Budowanie wersji produkcyjnej:**
   ```bash
   $env:API_PROXY_KEY=""; npm run tauri build --release
   ```

---

## 📄 Licencja

Projekt udostępniany na licencji MIT. Szczegóły w pliku `LICENSE`.

---
*Stworzone z pasją do efektywnej nauki języków.*
