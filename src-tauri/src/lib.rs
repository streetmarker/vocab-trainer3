// src-tauri/src/lib.rs
// All modules declared here. main.rs just calls run().

pub mod db;
pub mod learning;
pub mod commands;

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
pub fn show_task_notification(app: &tauri::AppHandle, word: &db::Word) {
    const LABEL: &str = "task-notification";
    const WIN_W: f64  = 360.0;
    const WIN_H: f64  = 138.0;

    let title       = word.term.clone();
    let description = word.definition.chars().take(90).collect::<String>();
    let word_id     = word.id;

    // ── Get existing window or create it now ─────────────────────────────────
    let notif = if let Some(w) = app.get_webview_window(LABEL) {
        w // warm: already loaded, just parked off-screen
    } else {
        // First trigger: create the window. Points to notification.html which
        // loads notification.tsx — a dedicated entry, not shared with main.
        match tauri::WebviewWindowBuilder::new(
            app,
            LABEL,
            tauri::WebviewUrl::App("notification.html".into()),
        )
        .title("")
        .inner_size(WIN_W, WIN_H)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false) // hidden until repositioned below
        .build()
        {
            Ok(w)  => w,
            Err(e) => {
                log::error!("show_task_notification: build failed: {}", e);
                return;
            }
        }
    };

    // ── Position bottom-right, above taskbar ─────────────────────────────────
    if let Ok(Some(monitor)) = notif.primary_monitor() {
        let screen  = monitor.size();
        let scale   = monitor.scale_factor();
        let win_w   = (WIN_W * scale) as u32;
        let win_h   = (WIN_H * scale) as u32;
        let taskbar = (48.0  * scale) as u32;
        let margin  = (16.0  * scale) as u32;
        let x = screen.width.saturating_sub(win_w + margin) as i32;
        let y = screen.height.saturating_sub(win_h + taskbar + margin) as i32;
        let _ = notif.set_size(PhysicalSize::new(win_w, win_h));
        let _ = notif.set_position(PhysicalPosition::new(x, y));
    }
    let _ = notif.set_always_on_top(true);
    let _ = notif.show(); // un-throttles WebView2 if it was parked

    // ── Emit after 500ms ──────────────────────────────────────────────────────
    // On first creation: React needs ~400ms to mount and register the listener.
    // On subsequent shows (after hide()): WebView2 needs time to un-throttle
    // after being hidden. 500ms is safe for both cases.
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        use tauri::Emitter;
        let _ = app_clone.emit_to(
            LABEL,
            "task-notification",
            serde_json::json!({
                "title":       title,
                "description": description,
                "wordId":      word_id,
            }),
        );
        if let Some(w) = app_clone.get_webview_window(LABEL) {
            let _ = w.set_focus();
        }
    });
}


/// The window is always "visible" but parked at -2000,-2000 when idle — this prevents
/// the browser engine from throttling its JavaScript when hidden.
pub fn show_popup(app: &tauri::AppHandle, word_id: i64) {
    if let Some(state) = app.try_state::<commands::AppState>() {
        if let Ok(mut pending) = state.pending_word_id.lock() {
            *pending = Some(word_id);
        }
    }

    if let Some(popup) = app.get_webview_window("popup") {
        if let Ok(Some(monitor)) = popup.primary_monitor() {
            let screen  = monitor.size();
            let scale   = monitor.scale_factor();
            let win_w   = (360.0 * scale) as u32;
            let win_h   = (480.0 * scale) as u32;
            let taskbar = (48.0 * scale) as u32;
            let margin  = (12.0 * scale) as u32;
            let x = (screen.width.saturating_sub(win_w + margin)) as i32;
            let y = (screen.height.saturating_sub(win_h + taskbar)) as i32;
            let _ = popup.set_size(PhysicalSize::new(win_w, win_h));
            let _ = popup.set_position(PhysicalPosition::new(x, y));
        }
        let _ = popup.set_always_on_top(true);

        // Emit after 300ms — popup JS must register its listener first.
        // emit_to("popup", ...) targets WebviewWindow{label:"popup"} which matches
        // getCurrentWebviewWindow().listen() registered in popup.tsx.
        let app_clone = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            use tauri::Emitter;
            let _ = app_clone.emit_to(
                "popup",
                "load_exercise",
                serde_json::json!({ "wordId": word_id }),
            );
            if let Some(w) = app_clone.get_webview_window("popup") {
                let _ = w.set_focus();
            }
        });
    }
}


pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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
                work_hours_start:    saved.work_hours_start
                    .split(':').next()
                    .and_then(|h| h.parse().ok())
                    .unwrap_or(8),
                work_hours_end:      saved.work_hours_end
                    .split(':').next()
                    .and_then(|h| h.parse().ok())
                    .unwrap_or(22),
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
            {
                let db_clone = Arc::clone(&db);
                let handle   = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    if let Ok(Some((word, _))) = db_clone.get_session_word() {
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
            commands::add_word,
            commands::delete_word,
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
            commands::import_words_from_json,
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
