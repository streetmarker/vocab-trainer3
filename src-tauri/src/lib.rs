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
    Manager, WindowEvent,
};

use db::Database;
use learning::{LearningEngine, ProgressTracker, new_session_id};
use learning::scheduler::{Scheduler, SchedulerConfig};
use commands::AppState;


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
            let scheduler = Arc::new(Scheduler::new(
                Arc::clone(&db),
                SchedulerConfig::default(),
                session_id,
            ));

            app.manage(AppState {
                db: Arc::clone(&db),
                engine: Arc::clone(&engine),
                tracker: Arc::clone(&tracker),
                scheduler: Arc::clone(&scheduler),
                data_dir: data_dir.clone(),
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

            // Show session intro popup after 3s
            {
                let engine_clone = Arc::clone(&engine);
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    if let Ok(Some(_)) = engine_clone.start_session() {
                        use tauri::Emitter;
                        let _ = handle.emit("session_started", ());
                        if let Some(win) = handle.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
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
