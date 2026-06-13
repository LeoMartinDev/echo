mod audio;
mod engine;
mod focus;
mod history;
mod models;
mod session;
mod settings;
mod state;
mod typing;

use serde::Serialize;
use settings::AppSettings;
use state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Serialize)]
struct ModelInfo {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    engine: models::EngineKind,
    size_mb: u32,
    downloaded: bool,
    downloading: bool,
    active: bool,
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> AppSettings {
    state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[tauri::command]
fn set_settings(
    app: AppHandle,
    state: tauri::State<AppState>,
    new_settings: AppSettings,
) -> Result<(), String> {
    let old = {
        let mut guard = state.settings.lock().unwrap_or_else(|e| e.into_inner());
        let old = guard.clone();
        *guard = new_settings.clone();
        old
    };

    if old.hotkey != new_settings.hotkey {
        if let Err(e) = register_hotkey(&app, &new_settings.hotkey) {
            // Restore the previous shortcut so the app doesn't become unresponsive.
            let mut guard = state.settings.lock().unwrap_or_else(|e| e.into_inner());
            guard.hotkey = old.hotkey.clone();
            let _ = register_hotkey(&app, &old.hotkey);
            let _ = settings::save(&app, &guard);
            return Err(e);
        }
    }

    if old.autostart != new_settings.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let result =
            if new_settings.autostart { manager.enable() } else { manager.disable() };
        if let Err(e) = result {
            let mut guard = state.settings.lock().unwrap_or_else(|e| e.into_inner());
            guard.autostart = old.autostart;
            let _ = settings::save(&app, &guard);
            return Err(format!("Lancement automatique impossible : {e}"));
        }
    }

    // UI language changed: rebuild the tray menu in the new locale.
    if old.ui_language != new_settings.ui_language {
        apply_tray_locale(&app, settings::resolve_locale(&new_settings));
    }

    // Active model changed: drop the old engine and preload the new one
    // without blocking a dictation session that may be in progress.
    if old.model_id != new_settings.model_id {
        if let Ok(mut slot) = state.engine.try_lock() {
            slot.clear();
        }
        preload_engine(&app);
    }

    settings::save(&app, &new_settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_models(app: AppHandle, state: tauri::State<AppState>) -> Vec<ModelInfo> {
    let active_id = state.settings.lock().unwrap_or_else(|e| e.into_inner()).model_id.clone();
    let downloads = state.downloads.lock().unwrap_or_else(|e| e.into_inner());
    models::CATALOG
        .iter()
        .map(|spec| ModelInfo {
            id: spec.id,
            name: spec.name,
            description: spec.description,
            engine: spec.engine,
            size_mb: spec.size_mb,
            downloaded: models::is_downloaded(&app, spec),
            downloading: downloads.contains(spec.id),
            active: spec.id == active_id,
        })
        .collect()
}

#[tauri::command]
async fn download_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    {
        let mut downloads = state.downloads.lock().unwrap_or_else(|e| e.into_inner());
        if !downloads.insert(id.clone()) {
            return Err("Téléchargement déjà en cours.".to_string());
        }
    }
    let result = models::download(app.clone(), id.clone()).await;
    state
        .downloads
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&id);
    // If the active model just finished downloading, load it immediately.
    if result.is_ok() {
        let active = state
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .model_id
            .clone();
        if active == id {
            preload_engine(&app);
        }
    }
    result
}

#[tauri::command]
fn delete_model(app: AppHandle, state: tauri::State<AppState>, id: String) -> Result<(), String> {
    if let Ok(mut slot) = state.engine.try_lock() {
        if slot.model_id.as_deref() == Some(id.as_str()) {
            slot.clear();
        }
    }
    models::delete(&app, &id)
}

/// Resident memory (RSS) of the process in bytes — dominated by the loaded model.
#[tauri::command]
fn get_process_memory() -> u64 {
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let pid = Pid::from_u32(std::process::id());
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid).map(|p| p.memory()).unwrap_or(0)
}

#[tauri::command]
fn get_history(app: AppHandle) -> Vec<history::HistoryEntry> {
    history::list(&app)
}

#[tauri::command]
fn clear_history(app: AppHandle) -> Result<(), String> {
    history::clear(&app)
}

#[tauri::command]
fn delete_history_entry(app: AppHandle, ts_ms: u64) -> Result<(), String> {
    history::delete(&app, ts_ms)
}

/// macOS only: simulated keystrokes require the Accessibility permission.
#[tauri::command]
fn accessibility_status(prompt: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        if prompt {
            macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
        } else {
            macos_accessibility_client::accessibility::application_is_trusted()
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = prompt;
        true
    }
}

/// While the settings UI is capturing a new shortcut, unregister the global
/// hotkey so pressing the current one doesn't start a dictation. When capture
/// ends, re-arm the shortcut from the stored settings (this also covers the
/// case where the user re-selects the same key, which `set_settings` skips).
#[tauri::command]
fn set_hotkey_capture(
    app: AppHandle,
    state: tauri::State<AppState>,
    capturing: bool,
) -> Result<(), String> {
    if capturing {
        app.global_shortcut().unregister_all().map_err(|e| e.to_string())
    } else {
        let hotkey = state.settings.lock().unwrap_or_else(|e| e.into_inner()).hotkey.clone();
        register_hotkey(&app, &hotkey)
    }
}

fn register_hotkey(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let shortcut: Shortcut = accelerator
        .parse()
        .map_err(|e| format!("Raccourci invalide « {accelerator} » : {e}"))?;

    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    gs.on_shortcut(shortcut, |app, _shortcut, event| match event.state {
        ShortcutState::Pressed => session::start(app),
        ShortcutState::Released => session::stop(app),
    })
    .map_err(|e| format!("Impossible d'enregistrer « {accelerator} » : {e}"))?;
    Ok(())
}

/// Loads the active model into memory in the background (at launch, after a model
/// change or a download): the first shortcut press must not wait for loading. If
/// a dictation starts during loading, its worker simply waits on the mutex — the
/// audio capture is already running, so nothing is lost.
fn preload_engine(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let model_id = state
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .model_id
            .clone();
        let Some(spec) = models::spec(&model_id) else { return };
        if !models::is_downloaded(&app, spec) {
            return;
        }
        let mut slot = state.engine.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = slot.ensure_loaded(&app, &model_id) {
            eprintln!("[greffe] model preload failed: {e}");
        }
    });
}

fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

struct TrayLabels {
    open: &'static str,
    quit: &'static str,
    tooltip: &'static str,
}

fn tray_labels(locale: &str) -> TrayLabels {
    match locale {
        "fr" => TrayLabels {
            open: "Réglages…",
            quit: "Quitter Greffe",
            tooltip: "Greffe — dictée vocale",
        },
        _ => TrayLabels {
            open: "Settings…",
            quit: "Quit Greffe",
            tooltip: "Greffe — voice dictation",
        },
    }
}

fn build_tray_menu(app: &AppHandle, locale: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let l = tray_labels(locale);
    let open = MenuItem::with_id(app, "open", l.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open, &quit])
}

/// Updates the menu and tooltip of an already-installed tray icon (locale change).
fn apply_tray_locale(app: &AppHandle, locale: &str) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app, locale) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_tooltip(Some(tray_labels(locale).tooltip));
    }
}

fn setup_tray(app: &AppHandle, locale: &str) -> tauri::Result<()> {
    let menu = build_tray_menu(app, locale)?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip(tray_labels(locale).tooltip)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        // Windows: double-clicking the icon opens settings.
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                show_settings(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            list_models,
            download_model,
            delete_model,
            get_history,
            clear_history,
            delete_history_entry,
            get_process_memory,
            accessibility_status,
            set_hotkey_capture,
        ])
        .setup(|app| {
            // macOS: "agent" app — menu bar icon only, no Dock or Cmd+Tab,
            // as expected for a dictation utility.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let loaded = settings::load(&handle);
            let hotkey = loaded.hotkey.clone();
            let locale = settings::resolve_locale(&loaded);
            app.manage(AppState::new(loaded));

            setup_tray(&handle, locale)?;

            if let Err(e) = register_hotkey(&handle, &hotkey) {
                eprintln!("[greffe] {e}");
                // Corrupted or taken shortcut: fall back to the default.
                let fallback = AppSettings::default().hotkey;
                if register_hotkey(&handle, &fallback).is_ok() {
                    let state = handle.state::<AppState>();
                    state.settings.lock().unwrap_or_else(|e| e.into_inner()).hotkey =
                        fallback;
                }
            }

            // Model loaded at launch, not on first shortcut press.
            preload_engine(&handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window hides the app (it lives in the tray).
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            // The app only quits via "Quit" in the tray (app.exit).
            // Any other exit signal (Cmd+Q, window session close…) keeps it
            // running in the background to preserve the shortcut.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
