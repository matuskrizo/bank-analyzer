//! Vstupný bod aplikácie: zaregistruje pluginy, stav a príkazy.

mod categorize;
mod commands;
mod db;
mod error;
mod model;
mod parser;

use std::sync::Mutex;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Priečinok pre dáta aplikácie, napr. na macOS:
            // ~/Library/Application Support/sk.levelup.bank-analyzer/
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&data_dir.join("transakcie.db"))?;

            // `manage` uloží stav – príkazy si ho vypýtajú cez State<AppState>.
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_files,
            commands::list_transactions,
            commands::list_categories,
            commands::add_category,
            commands::list_rules,
            commands::add_rule,
            commands::delete_rule,
            commands::set_category,
            commands::list_imports,
            commands::delete_import,
        ])
        .run(tauri::generate_context!())
        .expect("chyba pri spúšťaní aplikácie");
}
