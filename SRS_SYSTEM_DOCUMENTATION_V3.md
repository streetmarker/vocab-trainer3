# 🧠 Vocab Trainer v3: Dokumentacja Systemu Smart-SRS

**Wersja:** 3.0 (Smart Repetition Engine)  
**Rola:** Senior Business Analyst / Full-Stack Architect  
**Status:** Produkcyjny (Wdrożony)

---

## 1. Wstęp i Cel Systemu

System **Smart-SRS** (Spaced Repetition System) w aplikacji Vocab Trainer v3 został zaprojektowany, aby zmaksymalizować efektywność zapamiętywania słownictwa przy minimalnym nakładzie czasu. System automatycznie zarządza kolejką nauki, dostosowując ją do indywidualnego tempa zapominania użytkownika oraz jego nieregularnego czasu pracy (np. sesje 3-4h dziennie).

---

## 2. Mechanizm Wyliczania Powtórek (Algorytm SM-2)

Sercem systemu jest zmodyfikowany algorytm **SuperMemo-2 (SM-2)**. Każda interakcja z fiszką aktualizuje parametry stabilności śladu pamięciowego.

### 2.1 Skala Oceny (Feedback) i Konsekwencje

Użytkownik ma do wyboru 4 opcje po odwróceniu fiszki:

| Ocena (UI) | Jakość (q) | Skutek Techniczny | Planowany Interwał |
| :--- | :--- | :--- | :--- |
| **Jeszcze raz** | 1 | **Błąd:** Reset `iterations` do 0. EF spada (-0.2). | Powrót za **1 dzień** |
| **Trudne** | 3 | **Pół-poprawne:** `iterations++`. EF spada (-0.1). | I_nowy = I_stary * 1.2 |
| **Dobrze** | 4 | **Poprawne:** `iterations++`. EF bez zmian. | I_nowy = I_stary * EF |
| **Łatwe** | 5 | **Idealne:** `iterations++`. EF rośnie (+0.1). | I_nowy = I_stary *EF* 1.3 |

### 2.2 Formuła Matematyczna

1. **EF (Easiness Factor):** Domyślnie 2.5. Parametr określający, jak szybko rośnie przerwa między powtórkami. Minimalny EF = 1.3.
2. **Interwał (dni):**
    - `n=1` (pierwsza poprawna): 1 dzień.
    - `n=2`: 6 dni.
    - `n>2`: `Przerwa = Poprzednia_Przerwa * EF`.

---

## 3. Zarządzanie Kolejką Dzienną (SQL Selection)

System filtruje bazę danych w sposób deterministyczny, co eliminuje błąd "pojawiania się słów tego samego dnia".

### 3.1 Hierarchia Wyboru

Gdy system szuka kolejnego słowa do wyświetlenia, stosuje następujący priorytet (Kolejka FIFO dla opóźnień):

1. **Słowa Zaległe (Critical):** `next_review_at <= NOW`. Wyświetlane w kolejności od najbardziej opóźnionego.
2. **Słowa Nowe (Introduction):** Jeśli brak słów zaległych, system wprowadza nowe słowa (zgodnie z dziennym limitem w ustawieniach).
3. **Słowa Zaplanowane:** Słowa, których termin przypada w przyszłości, są **całkowicie zablokowane** w trybie automatycznym.

### 3.2 System Ograniczeń (Cooldown)

Wprowadzono **Intra-day Cooldown (12h)**:

- Jeśli ocenisz słowo jako **Dobrze** lub **Łatwe**, system wymusza minimum **12 godzin przerwy**, nawet jeśli algorytm matematyczny sugerowałby krótszy czas (np. dla bardzo nowych słów).
- Gwarantuje to, że nie będziesz uczył się tego samego słowa dwukrotnie podczas jednej 3-4 godzinnej sesji przy komputerze.

---

## 4. Scenariusze Przykładowe

### Scenariusz A: Nowe, trudne słowo ("Latency")

1. **Dzień 1, 08:00:** Pierwsze widzenie. Klikasz "Rozumiem". (Interwał: 1d). `next_review`: Dzień 2, 08:00.
2. **Dzień 2, 09:00:** Słowo pojawia się w popupie. Klikasz "Trudne". EF spada. `next_review`: Dzień 3, ok. 10:00.
3. **Dzień 3:** Zapominasz słowo. Klikasz "Jeszcze raz". `iterations` resetuje się. Słowo wróci za 1 dzień.

### Scenariusz B: Znane słowo ("Concurrency")

1. **Poniedziałek, 12:00:** Widzisz słowo, klikasz "Łatwe". `next_review` ustawione na za 6 dni (Niedziela).
2. **Czwartek:** Uruchamiasz komputer. Słowo **nie pojawi się**, ponieważ termin (Niedziela) jeszcze nie zapadł.
3. **Niedziela, 18:00:** Włączasz WiFi. Słowo pojawia się jako pierwsza powtórka dnia.

---

## 5. Przejrzystość Interfejsu (Visual Feedback)

W wersji v3 użytkownik widzi dokładny plan systemu:

- Pod każdą odpowiedzią wyświetla się informacja: `Następna powtórka: dzisiaj 22:45` lub `Następna powtórka: Pt, 08:00`.
- Eliminuje to niepewność użytkownika co do działania algorytmu.

---

## 6. Uwagi Analityka (Future Insights)

- **Kompatybilność z Offline:** System zapisuje daty lokalnie. Jeśli nie masz internetu, SRS nadal działa poprawnie (nie wymaga synchronizacji czasu z serwerem).
- **Manual Study:** Przycisk "Ćwicz teraz" na Dashboardzie służy jako "override". Pozwala on na naukę słów, które jeszcze nie są zaległe (np. przed egzaminem), bez psucia głównego harmonogramu SRS.

---
Dokumentacja stworzona przez Senior Business Analyst (AI Assisted)*
