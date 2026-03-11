// src-tauri/src/learning/scheduler.rs

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use chrono::{Timelike, Utc};
use parking_lot::RwLock;
use tokio::time;

use crate::db::{Database, Word, WordProgress};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub idle_threshold_secs: u64,
    pub min_popup_gap_secs: u64,
    pub poll_interval_secs: u64,
    pub max_daily_exercises: i32,
    pub work_hours_start: u32,
    pub work_hours_end: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            idle_threshold_secs: 5,
            min_popup_gap_secs: 30 * 60,
            poll_interval_secs: 10,
            max_daily_exercises: 50,
            work_hours_start: 8,
            work_hours_end: 22,
        }
    }
}

// ─── State ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct SchedulerState {
    pub last_popup_at: Option<Instant>,
    pub session_id: String,
    pub exercises_today: i32,
    pub is_paused: bool,
    pub current_word: Option<(Word, WordProgress)>,
}

impl SchedulerState {
    pub fn new(session_id: String) -> Self {
        Self {
            last_popup_at: None,
            session_id,
            exercises_today: 0,
            is_paused: false,
            current_word: None,
        }
    }
}

// ─── Conditions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PopupConditions {
    pub user_is_idle: bool,
    pub no_fullscreen: bool,
    pub enough_time_since_last: bool,
    pub within_work_hours: bool,
    pub not_paused: bool,
    pub has_due_exercises: bool,
    pub under_daily_limit: bool,
}

impl PopupConditions {
    pub fn all_met(&self) -> bool {
        self.user_is_idle
            && self.no_fullscreen
            && self.enough_time_since_last
            && self.within_work_hours
            && self.not_paused
            && self.has_due_exercises
            && self.under_daily_limit
    }

    pub fn reason_blocked(&self) -> Option<&'static str> {
        if !self.user_is_idle           { return Some("user active"); }
        if !self.no_fullscreen          { return Some("fullscreen app"); }
        if !self.enough_time_since_last { return Some("too soon"); }
        if !self.within_work_hours      { return Some("outside work hours"); }
        if !self.not_paused             { return Some("paused"); }
        if !self.has_due_exercises      { return Some("no due exercises"); }
        if !self.under_daily_limit      { return Some("daily limit reached"); }
        None
    }
}

// ─── Activity Detector ────────────────────────────────────────────────────────

pub struct ActivityDetector {
    last_known_idle_secs: u64,
}

impl ActivityDetector {
    pub fn new() -> Self {
        Self { last_known_idle_secs: 0 }
    }

    /// Returns idle seconds since last user input.
    #[cfg(target_os = "windows")]
    pub fn idle_seconds(&mut self) -> u64 {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        use windows::Win32::System::SystemInformation::GetTickCount;

        unsafe {
            let mut lii = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            // GetLastInputInfo returns Result<()> in windows crate 0.58+
            if GetLastInputInfo(&mut lii).as_bool() {
                let tick_count = GetTickCount();
                let idle_ms = tick_count.saturating_sub(lii.dwTime);
                self.last_known_idle_secs = (idle_ms / 1000) as u64;
            }
        }
        self.last_known_idle_secs
    }

    #[cfg(not(target_os = "windows"))]
    pub fn idle_seconds(&mut self) -> u64 {
        // Dev stub: always report 10s idle so exercises trigger during development
        10
    }

    /// Returns true if a fullscreen app is covering the entire monitor.
    #[cfg(target_os = "windows")]
    pub fn is_fullscreen_active(&self) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
        use windows::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        };
        use windows::Win32::Foundation::RECT;

        unsafe {
            let hwnd = GetForegroundWindow();
            // HWND.0 is *mut c_void — compare to null pointer
            if hwnd.0.is_null() { return false; }

            let mut win_rect = RECT::default();
            if GetWindowRect(hwnd, &mut win_rect).is_err() { return false; }

            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut mi).as_bool() {
                let mr = mi.rcMonitor;
                return win_rect.left   <= mr.left
                    && win_rect.top    <= mr.top
                    && win_rect.right  >= mr.right
                    && win_rect.bottom >= mr.bottom;
            }
        }
        false
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_fullscreen_active(&self) -> bool {
        false
    }
}

// ─── Next Exercise Selector ───────────────────────────────────────────────────

pub fn select_next_exercise(
    db: &Database,
    current_word: &Option<(Word, WordProgress)>,
) -> Result<Option<(Word, WordProgress)>> {
    let now = Utc::now();

    // In-session micro-interval due?
    if let Some((word, progress)) = current_word {
        if let Some(next_session) = progress.next_session_review_at {
            if next_session <= now {
                return Ok(Some((word.clone(), progress.clone())));
            }
        }
    }

    // SM-2 inter-day review due?
    let due = db.get_due_words()?;
    if !due.is_empty() {
        return Ok(Some(due.into_iter().next().unwrap()));
    }

    // Fallback: current session word
    if let Some(pair) = current_word {
        return Ok(Some(pair.clone()));
    }

    db.get_session_word()
}

// ─── Scheduler ────────────────────────────────────────────────────────────────

pub struct Scheduler {
    config: SchedulerConfig,
    state: Arc<RwLock<SchedulerState>>,
    db: Arc<Database>,
    detector: Arc<parking_lot::Mutex<ActivityDetector>>,
}

impl Scheduler {
    pub fn new(db: Arc<Database>, config: SchedulerConfig, session_id: String) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(SchedulerState::new(session_id))),
            db,
            detector: Arc::new(parking_lot::Mutex::new(ActivityDetector::new())),
        }
    }

    pub fn state(&self) -> Arc<RwLock<SchedulerState>> {
        Arc::clone(&self.state)
    }

    pub fn check_conditions(&self) -> PopupConditions {
        let state = self.state.read();
        let hour = chrono::Local::now().hour();
        let idle_secs = self.detector.lock().idle_seconds();
        let fullscreen = self.detector.lock().is_fullscreen_active();

        let enough_time = state
            .last_popup_at
            .map(|t| t.elapsed() >= Duration::from_secs(self.config.min_popup_gap_secs))
            .unwrap_or(true);

        PopupConditions {
            user_is_idle: idle_secs >= self.config.idle_threshold_secs,
            no_fullscreen: !fullscreen,
            enough_time_since_last: enough_time,
            within_work_hours: hour >= self.config.work_hours_start
                && hour < self.config.work_hours_end,
            not_paused: !state.is_paused,
            has_due_exercises: true,
            under_daily_limit: state.exercises_today < self.config.max_daily_exercises,
        }
    }

    pub fn record_popup_shown(&self) {
        let mut state = self.state.write();
        state.last_popup_at = Some(Instant::now());
        state.exercises_today += 1;
    }

    pub fn set_paused(&self, paused: bool) {
        self.state.write().is_paused = paused;
    }

    pub fn set_current_word(&self, word: Option<(Word, WordProgress)>) {
        self.state.write().current_word = word;
    }

    pub fn session_id(&self) -> String {
        self.state.read().session_id.clone()
    }

    pub async fn run(self: Arc<Self>, app_handle: tauri::AppHandle) {
        let poll = Duration::from_secs(self.config.poll_interval_secs);
        loop {
            time::sleep(poll).await;

            let mut conditions = self.check_conditions();
            let current_word = self.state.read().current_word.clone();
            let next_exercise = select_next_exercise(&self.db, &current_word);
            conditions.has_due_exercises = next_exercise
                .as_ref()
                .map(|o| o.is_some())
                .unwrap_or(false);

            if conditions.all_met() {
                if let Ok(Some((word, progress))) = next_exercise {
                    self.record_popup_shown();
                    use tauri::Emitter;
                    let _ = app_handle.emit("show_exercise", serde_json::json!({
                        "wordId": word.id,
                        "sessionId": self.state.read().session_id,
                    }));
                    log::info!(
                        "Scheduler: showing '{}' (mastery: {:?})",
                        word.term, progress.mastery_level
                    );
                }
            } else if let Some(reason) = conditions.reason_blocked() {
                log::trace!("Scheduler blocked: {}", reason);
            }
        }
    }
}
