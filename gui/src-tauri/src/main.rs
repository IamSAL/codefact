//! codefacts tray app (Tauri v2). A thin front-end over the CLI + core: the tray
//! menu triggers `codefacts …`, the settings/messages window reads & writes the
//! shared config via `codefacts-core`. It never schedules — the worker/daemon does.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

use codefacts_core::config::Config;
use codefacts_core::paths;
use codefacts_core::secrets::Secrets;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

fn run_codefacts(args: &[&str]) {
    let _ = Command::new("codefacts").args(args).spawn();
}

fn show_window(app: &tauri::AppHandle, tab: &str) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.eval(&format!("window.__cfTab && window.__cfTab('{tab}')"));
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Config + secret presence for the settings form (token itself is redacted).
#[tauri::command]
fn load_settings() -> Result<Value, String> {
    let cfg = Config::load(paths::config_path().map_err(e)?).map_err(e)?;
    let repo = cfg.repos.first();
    let secrets = paths::secrets_path().ok().and_then(|p| Secrets::load(p).ok());
    Ok(json!({
        "times": cfg.schedule.times.join(","),
        "timezone": cfg.schedule.timezone,
        "repo": repo.map(|r| r.path.clone()).unwrap_or_default(),
        "interest": repo.map(|r| r.interest.clone()).unwrap_or_default(),
        "enabled": cfg.enabled,
        "telegram_configured": secrets.is_some(),
        "token_hint": secrets.map(|s| s.redacted_token()).unwrap_or_default(),
    }))
}

#[derive(Deserialize)]
struct SettingsIn {
    times: String,
    repo: String,
    interest: String,
    token: Option<String>,
    chat_id: Option<String>,
}

#[tauri::command]
fn save_settings(input: SettingsIn) -> Result<(), String> {
    let mut cfg = Config::load(paths::config_path().map_err(e)?).map_err(e)?;
    cfg.schedule.times = input.times.split(',').map(|s| s.trim().to_string()).collect();
    if let Some(r) = cfg.repos.first_mut() {
        r.path = input.repo;
        r.interest = input.interest;
    }
    cfg.validate().map_err(e)?;
    cfg.save(paths::config_path().map_err(e)?).map_err(e)?;

    // Only rewrite secrets if a new token was actually provided.
    if let (Some(token), Some(chat)) = (input.token, input.chat_id) {
        if !token.is_empty() {
            Secrets {
                telegram_bot_token: token,
                telegram_chat_id: chat,
            }
            .save(paths::secrets_path().map_err(e)?)
            .map_err(e)?;
        }
    }
    Ok(())
}

/// Raw dump of recent emissions via the engine (falls back to a hint).
#[tauri::command]
fn list_messages() -> Result<String, String> {
    let cfg = Config::load(paths::config_path().map_err(e)?).map_err(e)?;
    let Some(repo) = cfg.repos.first() else {
        return Ok("No repo configured.".into());
    };
    let scope = format!("cf:{}:emissions", paths::repo_id(&repo.path));
    let out = Command::new("iii")
        .args(["trigger", "state::list", "--json", &json!({ "scope": scope }).to_string()])
        .output()
        .map_err(e)?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[tauri::command]
fn toggle_enabled() -> Result<bool, String> {
    let mut cfg = Config::load(paths::config_path().map_err(e)?).map_err(e)?;
    cfg.enabled = !cfg.enabled;
    cfg.save(paths::config_path().map_err(e)?).map_err(e)?;
    Ok(cfg.enabled)
}

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            list_messages,
            toggle_enabled
        ])
        .setup(|app| {
            let emit = MenuItem::with_id(app, "emit", "Emit now", true, None::<&str>)?;
            let mine = MenuItem::with_id(app, "mine", "Mine now", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let messages = MenuItem::with_id(app, "messages", "View messages…", true, None::<&str>)?;
            let console =
                MenuItem::with_id(app, "console", "Open Observability Console", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pause / Resume", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&emit, &mine, &sep1, &settings, &messages, &console, &pause, &sep2, &quit],
            )?;

            let mut builder = TrayIconBuilder::new().menu(&menu).show_menu_on_left_click(true);
            if let Some(icon) = app.default_window_icon().cloned() {
                builder = builder.icon(icon);
            }
            builder
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "emit" => run_codefacts(&["emit"]),
                    "mine" => run_codefacts(&["mine"]),
                    "console" => run_codefacts(&["console"]),
                    "settings" => show_window(app, "settings"),
                    "messages" => show_window(app, "messages"),
                    "pause" => {
                        let _ = toggle_enabled();
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running codefacts tray");
}
