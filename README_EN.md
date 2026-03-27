# 🇵🇱 [Polski](README.md) | 🇺🇸 [English](README_EN.md)

# 🧠 Vocab Trainer v3

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19.0-61dafb)](https://react.dev/)

**Vocab Trainer v3** is an advanced desktop application for vocabulary learning, powered by the scientifically proven **Spaced Repetition System (SRS)**. It addresses the lack of consistency and the organizational challenges of physical flashcards by automating the review process and reminding you to study at the most optimal moments.

---

## 🚀 The Problem & The Solution

Traditional learning with physical flashcards requires immense discipline and complex sorting logic (e.g., the Leitner system). Most learners give up because of:
- **Lack of Organization:** It is difficult to track which word needs a review today versus a week from now.
- **Lack of Motivation:** Physical cards don't "nudge" you when you forget them.
- **Inconsistency:** Daily life makes it hard to maintain a steady learning rhythm.

**Vocab Trainer v3** eliminates these barriers by acting as your personal language mentor tucked away in your system tray. The app manages your schedule automatically, displaying discreet notifications and popups precisely when your memory trace begins to fade.

---

## 📸 Application Showcase

### 1. Main Dashboard
The command center for your learning journey. Here you can track your statistics, overall progress, and see the list of upcoming reviews.
> ![Dashboard Screenshot](/panel.jpg)

### 2. Intelligent Review Popup
The core feature of the app. It appears automatically, presenting words for review. Thanks to system-level integration, you don't need to remember to launch the app manually.
> ![Popup Front Screenshot](/fiszka-front.jpg)
> ![Popup Back Screenshot](/fiszka-back.jpg)

---

## 🛠 Technical Architecture

The application is built using the modern **Tauri** architecture, combining the performance of Rust with the flexibility of React.

### Tech Stack:
- **Frontend:** React 19 + TypeScript + Vite
- **Desktop Core:** Tauri v2 (Rust-based security & performance)
- **Audio Service:** API Proxy for **Google TTS (Text-to-Speech)** hosted on **Google Cloud** (provides natural-sounding pronunciation)
- **Styling:** Vanilla CSS (Custom Optimized Styles)
- **Algorithm:** Modified **SM-2 (SuperMemo-2)** for intelligent interval calculation
- **Storage:** Local JSON System (deterministic handling of the flashcard database)

---

## 🤖 AI-Driven Development

This application is not the result of a traditional coding process but rather of advanced prompt engineering and collaboration with LLMs.

- **Framework:** The entire development lifecycle was guided by the **AI Fluency** framework, ensuring top-tier code quality and architectural consistency.
- **Tools:** The project was built using **Claude 3.5 Sonnet** (business logic and UI) and **Gemini CLI** (documentation automation, refactoring, and codebase analysis).
- **Methodology:** Leveraging "Systematic Prompting" to generate deterministic components and secure communication with the Tauri system APIs.

### Key Features:
- **Autostart:** The application launches with the system, running silently in the background.
- **Multi-window:** Separate modules for Dashboard, Popups, and Notifications.
- **Offline First:** Full functionality without an internet connection.
- **Smart Queue:** Intra-day Cooldown (12h) to prevent "overlearning" the same words in short sessions.

---

## 🏗 Installation & Setup

To run the project locally, ensure you have the [Tauri](https://tauri.app/v1/guides/getting-started/prerequisites) environment installed.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-repo/vocab-trainer3.git
   cd vocab-trainer3
   ```

2. **Install dependencies:**
   ```bash
   npm install
   ```

3. **Run in development mode:**
   ```bash
   npm run tauri dev
   ```

4. **Build the production version:**
   ```bash
   npm run tauri build
   ```

---

## 📄 License

This project is licensed under the MIT License. See the `LICENSE` file for details.

---
*Built with passion for efficient language learning.*
