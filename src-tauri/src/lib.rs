// src-tauri/src/lib.rs
// All modules declared here. main.rs just calls run().

pub mod db;
pub mod learning;
pub mod commands;
pub mod tts;

use std::sync::Arc;
use anyhow::Result;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    Manager, WindowEvent, PhysicalPosition, PhysicalSize,
};

use db::Database;
use learning::{LearningEngine, ProgressTracker, new_session_id};
use learning::scheduler::{Scheduler, SchedulerConfig};
use commands::AppState;
use dotenvy::dotenv;

/// Show the task-notification toast window.
///
/// Pattern mirrors show_popup() exactly:
///   - First call: create window with WebviewWindowBuilder pointing to notification.html
///     (its own HTML+JS bundle, no shared entry with main window)
///   - Subsequent calls: window exists but is parked at -2000,-2000
///     (notification.tsx parks via setPosition, NOT hide, to keep JS alive)
///   - Always: reposition to bottom-right, show(), then emit event after 300ms
///
/// Using notification.html (not index.html) is critical — it ensures:
///   1. The window has its own isolated JS context (no label-routing needed)
///   2. getCurrentWebviewWindow() returns "task-notification" unambiguously
///   3. JS is never shared with or affected by the main window's React tree
/// Parse "HH:MM" string into minutes from midnight.
/// Falls back to `default_mins` on any parse error.
/// Examples: "08:30" → 510,  "22:00" → 1320,  "08" (no colon) → 480
pub fn parse_hhmm_to_mins(s: &str, default_mins: u32) -> u32 {
    let mut parts = s.splitn(2, ':');
    let h: u32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(default_mins / 60);
    let m: u32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    h * 60 + m
}

pub fn show_task_notification(app: &tauri::AppHandle, word: &db::Word) {
    const LABEL: &str = "task-notification";

    log::info!("[notif] show_task_notification called for '{}'", word.term);

    // ── Compute logical window size from primary monitor ──────────────────────
    // Target: 22% of logical screen width, 35% of logical screen height
    // "Logical px" = CSS px = physical px / scale_factor.
    // Window scales proportionally with screen DPI and resolution.
    //
    // Clamping ensures:
    //   - Min width: 300px (readability threshold)
    //   - Max width: 480px (don't take up too much screen)
    //   - Min height: 200px (minimum content space)
    //   - Max height: 600px (don't dominate screen)

    // ── Adaptive Window Engine (Proposal 2) ──────────────────────────────────
    const BASE_WIDTH: f64 = 380.0;
    const BASE_HEIGHT: f64 = 280.0;
    
    let (win_w_log, win_h_log) = if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let phys_w = monitor.size().width as f64;
        let logical_screen_w = phys_w / scale;

        // Baza skalowana o DPI (które odzwierciedla rozmiar tekstu w Win11)
        let mut target_w = BASE_WIDTH;
        let mut target_h = BASE_HEIGHT;

        // Clamp: okno nie może zająć mniej niż 20% i więcej niż 45% szerokości ekranu
        let min_w = logical_screen_w * 0.20;
        let max_w = logical_screen_w * 0.45;
        target_w = target_w.clamp(min_w, max_w);
        target_h = target_h * (target_w / BASE_WIDTH);

        (target_w, target_h)
    } else {
        (BASE_WIDTH, BASE_HEIGHT)
    };

    // termPl = Polish definition shown bold on flashcard front
    let term_pl        = word.definition_pl.clone()
        .unwrap_or_else(|| word.definition.chars().take(60).collect());
    let term_en        = word.term.clone();
    let part_of_speech = word.part_of_speech.clone();
    let phonetic       = word.phonetic.clone();
    let sentence_pl    = word.sentence_pl.clone();
    let sentence_en    = word.sentence_en.clone();
    let word_id        = word.id;

    // ── Get existing window or create it now ─────────────────────────────────
    let notif = if let Some(w) = app.get_webview_window(LABEL) {
        log::info!("[notif] window warm (reusing existing)");
        w
    } else {
        // First trigger: create the window. Points to notification.html which
        // loads notification.tsx — a dedicated entry, not shared with main.
        match tauri::WebviewWindowBuilder::new(
            app,
            LABEL,
            tauri::WebviewUrl::App("notification.html".into()),
        )
        .title("")
        .inner_size(win_w_log, win_h_log)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false) // hidden until repositioned below
        .build()
        {
            Ok(w)  => {
                log::info!("[notif] window cold (created fresh)");
                w
            }
            Err(e) => {
                log::error!("show_task_notification: build failed: {}", e);
                return;
            }
        }
    };

    // ── Size and position bottom-right, above taskbar (always, warm or cold) ──
    if let Ok(Some(monitor)) = notif.primary_monitor() {
        let screen  = monitor.size();           // PhysicalSize
        let scale   = monitor.scale_factor();
        let win_w   = (win_w_log * scale).round() as u32;
        let win_h   = (win_h_log * scale).round() as u32;
        let taskbar = (48.0 * scale).round() as u32;
        let margin  = (16.0 * scale).round() as u32;
        let x = screen.width.saturating_sub(win_w + margin) as i32;
        let y = screen.height.saturating_sub(win_h + taskbar + margin) as i32;
        let _ = notif.set_size(PhysicalSize::new(win_w, win_h));
        let _ = notif.set_position(PhysicalPosition::new(x, y));
    }
    let _ = notif.set_always_on_top(true);

    // ── Prevent focus steal: set WS_EX_NOACTIVATE on the OS window ────────────
    // Tauri's `focus: false` is a known no-op on Windows (tauri#7519).
    // The only reliable fix is patching WS_EX_NOACTIVATE directly via Win32.
    //
    // WHY raw extern instead of the `windows` crate:
    //   Tauri v2 internally pulls windows-core 0.61.x; our Cargo.toml pins
    //   windows = "0.58".  The HWND returned by notif.hwnd() comes from Tauri's
    //   copy of windows-core, so it doesn't satisfy the Param<HWND> bound of
    //   0.58's GetWindowLongW — causing a type-mismatch compile error.
    //   Declaring the functions ourselves via #[link(name="user32")] bypasses
    //   the crate entirely: we just pass hwnd.0 (isize) directly.
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetWindowLongW(hwnd: *mut std::ffi::c_void, n_index: i32) -> i32;
            fn SetWindowLongW(hwnd: *mut std::ffi::c_void, n_index: i32, dw_new_long: i32) -> i32;
        }
        const GWL_EXSTYLE:      i32 = -20;
        const WS_EX_NOACTIVATE: i32 = 0x0800_0000_u32 as i32;

        if let Ok(hwnd) = notif.hwnd() {
            // HWND.0 is *mut c_void in Tauri's windows-core 0.61
            unsafe {
                let raw = hwnd.0;
                let ex  = GetWindowLongW(raw, GWL_EXSTYLE);
                SetWindowLongW(raw, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE);
            }
        }
    }

    log::info!("[notif] calling show() on window");
    let _ = notif.show(); // un-throttles WebView2 if it was parked

    // ── Emit after 500ms ──────────────────────────────────────────────────────
    // On first creation: React needs ~400ms to mount and register the listener.
    // On subsequent shows (after hide()): WebView2 needs time to un-throttle
    // after being hidden. 500ms is safe for both cases.
    // NOTE: set_focus() intentionally removed — the notification must NEVER
    //       interrupt the user's active window.
    let word_term_log = term_en.clone();
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        use tauri::Emitter;
        log::info!("[notif] emitting payload for '{word_term_log}' to window");
        let result = app_clone.emit_to(
            LABEL,
            "task-notification",
            serde_json::json!({
                "termPl":       term_pl,
                "termEn":       term_en,
                "partOfSpeech": part_of_speech,
                "phonetic":     phonetic,
                "sentencePl":   sentence_pl,
                "sentenceEn":   sentence_en,
                "wordId":       word_id,
            }),
        );
        if let Err(e) = result {
            log::error!("[notif] emit_to failed: {e}");
        } else {
            log::info!("[notif] payload emitted OK");
        }
        // set_focus() deliberately omitted — WS_EX_NOACTIVATE handles focus isolation
    });
}

pub fn show_popup(app: &tauri::AppHandle, word_id: i64) {
    // 1. Zapisujemy ID słowa w stanie aplikacji
    if let Some(state) = app.try_state::<commands::AppState>() {
        if let Ok(mut pending) = state.pending_word_id.lock() {
            *pending = Some(word_id);
        }
    }

    // ── Adaptive Window Engine (Proposal 2) ──────────────────────────────────
    const BASE_WIDTH: f64 = 420.0;
    const BASE_HEIGHT: f64 = 700.0;

    let (win_w, win_h) = if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let phys_w = monitor.size().width as f64;
        let logical_screen_w = phys_w / scale;

        let mut target_w = BASE_WIDTH;
        
        // Clamp: okno nie może zająć mniej niż 20% i więcej niż 45% szerokości ekranu
        let min_w = logical_screen_w * 0.20;
        let max_w = logical_screen_w * 0.45;
        target_w = target_w.clamp(min_w, max_w);
        
        let target_h = BASE_HEIGHT * (target_w / BASE_WIDTH);
        (target_w, target_h)
    } else {
        (BASE_WIDTH, BASE_HEIGHT)
    };

    // 2. Dynamiczne budowanie okna
    let popup_window = tauri::WebviewWindowBuilder::new(
        app,
        "popup",
        tauri::WebviewUrl::App("popup.html".into()),
    )
    .title("Ćwiczenie")
    .inner_size(win_w, win_h)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .visible(true)
    .zoom_hotkeys_enabled(false)
    .build();

    if let Ok(w) = popup_window {
        let _ = w.set_zoom(1.0);
    }

    // 3. Emitujemy zdarzenie po krótkim czasie (cold-start delay)
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600)); // Zwiększony czas na start procesu
        use tauri::Emitter;
        let _ = app_clone.emit_to("popup", "load_exercise", serde_json::json!({ "wordId": word_id }));
    });
}


pub fn run() {
    // Załaduj plik .env (jeśli istnieje) do std::env
    // Spróbuj załadować z src-tauri/.env (uruchamiane z katalogu głównego projektu)
    dotenvy::from_filename("src-tauri/.env").ok();
    // Backup: spróbuj z bieżącego katalogu (na wypadek uruchomienia bezpośrednio z src-tauri)
    dotenv().ok();

    // Debug: sprawdź czy zmienna została załadowana
    if let Ok(creds) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        log::info!("GOOGLE_APPLICATION_CREDENTIALS loaded: {}", creds);
    } else {
        log::warn!("GOOGLE_APPLICATION_CREDENTIALS not found in environment");
    }

    // Inicjalizuj logger tylko w trybie debug, aby uniknąć konsoli w Release
    #[cfg(debug_assertions)]
    {
        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("info"),
        )
        .format(|buf, record| {
            use std::io::Write;
            let ts = chrono::Local::now().format("%H:%M:%S%.3f");
            writeln!(buf, "[{ts}] {:<5} {}", record.level(), record.args())
        })
        .init();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("no data dir");
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("vocab_trainer.db");
            log::info!("Database path: {:?}", db_path);

            let db = Arc::new(Database::new(db_path)?);
            let engine = Arc::new(LearningEngine::new(Arc::clone(&db)));
            let tracker = Arc::new(ProgressTracker::new(Arc::clone(&db)));
            let session_id = new_session_id();

            // ── Load persisted settings → apply to SchedulerConfig ────────────
            // Without this the scheduler always uses hardcoded defaults (30 min
            // gap) regardless of what the user saved in the Settings page.
            let saved: commands::AppSettings = {
                let path = data_dir.join("settings.json");
                std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default()
            };
            let scheduler_config = SchedulerConfig {
                idle_threshold_secs: saved.idle_threshold_secs as u64,
                min_popup_gap_secs:  (saved.min_gap_minutes as u64) * 60,
                poll_interval_secs:  10,
                max_daily_exercises: saved.exercises_per_day as i32,
                work_hours_start:    parse_hhmm_to_mins(&saved.work_hours_start, 8 * 60),
                work_hours_end:      parse_hhmm_to_mins(&saved.work_hours_end,  22 * 60),
            };
            log::info!(
                "Scheduler: gap={}min idle={}s daily={} hours={}–{}",
                scheduler_config.min_popup_gap_secs / 60,
                scheduler_config.idle_threshold_secs,
                scheduler_config.max_daily_exercises,
                scheduler_config.work_hours_start,
                scheduler_config.work_hours_end,
            );

            let scheduler = Arc::new(Scheduler::new(
                Arc::clone(&db),
                scheduler_config,
                session_id,
            ));

            app.manage(AppState {
                db: Arc::clone(&db),
                engine: Arc::clone(&engine),
                tracker: Arc::clone(&tracker),
                scheduler: Arc::clone(&scheduler),
                data_dir: data_dir.clone(),
                pending_word_id: std::sync::Mutex::new(None),
            });

            setup_tray(app)?;

            // Start hidden in tray
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            // Scheduler background loop
            let handle = app.handle().clone();
            let sched = Arc::clone(&scheduler);
            tauri::async_runtime::spawn(async move {
                sched.run(handle).await;
            });

            // First notification 5s after startup.
            // ⚠ MUST use show_task_notification — NOT show_popup.
            // show_popup repositions the 480px popup window onto the screen, which
            // appears as a permanent transparent background behind the toast.
            // Bug fix: call record_popup_showing() BEFORE show_task_notification so
            // the scheduler loop sees last_popup_at != None and won't fire a second
            // notification in the very next 10-second poll (startup race).
            {
                let db_clone    = Arc::clone(&db);
                let handle      = app.handle().clone();
                let sched_clone = Arc::clone(&scheduler);
                tauri::async_runtime::spawn(async move {
                    // Czekamy 90 sekund, aż system w pełni "wstanie" (WiFi, usługi)
                    tokio::time::sleep(std::time::Duration::from_secs(90)).await;
                    if let Ok(Some((word, _))) = db_clone.get_session_word() {
                        sched_clone.record_popup_showing(); // blocks scheduler loop
                        show_task_notification(&handle, &word);
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_exercise,
            commands::submit_answer,
            commands::start_session,
            commands::get_words,
            commands::get_srs_overview,
            commands::add_word,
            commands::delete_word,
            commands::clear_words,
            commands::get_overall_stats,
            commands::get_daily_stats,
            commands::get_activity_grid,
            commands::get_scheduler_status,
            commands::set_scheduler_paused,
            commands::seed_sample_words,
            commands::get_settings,
            commands::save_settings,
            commands::get_popup_exercise,
            commands::hide_popup,
            commands::trigger_popup,
            commands::get_current_word,
            commands::task_notification_done,
            commands::task_notification_later,
            commands::task_notification_known,
            commands::flashcard_answer,
            commands::srs_answer,
            commands::initialize_autostart,
            commands::import_words_from_json,
            commands::get_struggling_words,
            commands::get_mentor_tips,
            commands::save_mentor_tips,
            tts::play_or_generate_tts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
fn setup_tray(app: &mut tauri::App) -> Result<()> {
    let open_dashboard = MenuItem::with_id(app, "dashboard", "Open Dashboard", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause Exercises", true, None::<&str>)?;
    let vocab = MenuItem::with_id(app, "vocab", "Vocabulary", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_dashboard, &pause, &vocab, &separator, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "dashboard" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                    use tauri::Emitter;
                    let _ = win.emit("navigate", "dashboard");
                }
            }
            "pause" => {
                let state = app.state::<AppState>();
                let is_paused = state.scheduler.state().read().is_paused;
                state.scheduler.set_paused(!is_paused);
            }
            "vocab" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                    use tauri::Emitter;
                    let _ = win.emit("navigate", "vocab");
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                if let Some(win) = tray.app_handle().get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
