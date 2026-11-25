//! Tauri príkazy – jediné miesto, kde sa Rust stretáva s frontendom.
//!
//! Každá funkcia s `#[tauri::command]` sa dá zavolať z JavaScriptu:
//! ```ts
//! await invoke("import_files", { paths: ["/cesta/vypis.csv"] })
//! ```
//! Názvy argumentov: v Ruste `snake_case`, v JS `camelCase`
//! (`transaction_id` ↔ `transactionId`) – Tauri ich prevedie samo.
//!
//! Príkazy sú zámerne tenké: len zamknú databázu a zavolajú `db`/`parser`.
//! Logika žije inde, aby sa dala testovať bez Tauri.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::model::{Category, ImportInfo, ImportReport, Rule, Transaction};
use crate::{db, parser};

/// Zdieľaný stav aplikácie. Tauri ho drží a príkazom ho požičiava
/// cez `State<'_, AppState>`. `Connection` nie je bezpečné zdieľať medzi
/// vláknami naraz, preto je v `Mutex`.
pub struct AppState {
    pub db: Mutex<Connection>,
}

impl AppState {
    fn db(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.db.lock().map_err(|_| AppError::LockPoisoned)
    }
}

/// Výsledok importu jedného súboru.
///
/// `tag = "status"` → v JSON pribudne pole `status` a TypeScript podľa neho
/// rozlíši varianty (tzv. discriminated union):
/// `{ status: "ok", fileName, imported, ... }` alebo
/// `{ status: "failed", fileName, error: { kind, message } }`.
#[derive(Debug, serde::Serialize)]
#[serde(
    tag = "status",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum ImportOutcome {
    Ok(ImportReport),
    Failed { file_name: String, error: AppError },
}

/// Import jedného alebo viacerých súborov (z drag-and-drop alebo dialógu).
///
/// `async` je dôležité: synchrónne príkazy bežia na hlavnom vlákne a počas
/// dlhého importu by "zamrzlo" okno. Asynchrónne bežia na pozadí.
/// Chyba v jednom súbore nezastaví ostatné – vrátime ju vo výsledku.
#[tauri::command]
pub async fn import_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> AppResult<Vec<ImportOutcome>> {
    let mut outcomes = Vec::new();
    for path in paths.into_iter().map(PathBuf::from) {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        // Parsovanie nepotrebuje databázu, takže zámok berieme až potom.
        let result = parser::parse_file(&path)
            .and_then(|parsed| db::import(&mut *state.db()?, &file_name, parsed));

        outcomes.push(match result {
            Ok(report) => ImportOutcome::Ok(report),
            Err(error) => ImportOutcome::Failed { file_name, error },
        });
    }
    Ok(outcomes)
}

#[tauri::command]
pub fn list_transactions(state: State<'_, AppState>) -> AppResult<Vec<Transaction>> {
    let conn = state.db()?;
    db::list_transactions(&conn)
}

#[tauri::command]
pub fn list_categories(state: State<'_, AppState>) -> AppResult<Vec<Category>> {
    let conn = state.db()?;
    db::list_categories(&conn)
}

#[tauri::command]
pub fn add_category(state: State<'_, AppState>, name: String) -> AppResult<Category> {
    let conn = state.db()?;
    db::add_category(&conn, &name)
}

#[tauri::command]
pub fn list_rules(state: State<'_, AppState>) -> AppResult<Vec<Rule>> {
    let conn = state.db()?;
    db::list_rules(&conn)
}

/// Pridá pravidlo a hneď podľa neho preradí nezaradené/automatické transakcie.
/// Vráti počet zmenených transakcií.
#[tauri::command]
pub fn add_rule(state: State<'_, AppState>, pattern: String, category_id: i64) -> AppResult<usize> {
    let mut conn = state.db()?;
    db::upsert_rule(&conn, &pattern, category_id)?;
    db::recategorize(&mut conn)
}

#[tauri::command]
pub fn delete_rule(state: State<'_, AppState>, rule_id: i64) -> AppResult<usize> {
    let mut conn = state.db()?;
    db::delete_rule(&conn, rule_id)?;
    db::recategorize(&mut conn)
}

/// Ručne nastaví kategóriu transakcie (`categoryId: null` ju vráti automatike).
#[tauri::command]
pub fn set_category(
    state: State<'_, AppState>,
    transaction_id: i64,
    category_id: Option<i64>,
) -> AppResult<()> {
    let conn = state.db()?;
    db::set_category(&conn, transaction_id, category_id)
}

#[tauri::command]
pub fn list_imports(state: State<'_, AppState>) -> AppResult<Vec<ImportInfo>> {
    let conn = state.db()?;
    db::list_imports(&conn)
}

#[tauri::command]
pub fn delete_import(state: State<'_, AppState>, import_id: i64) -> AppResult<()> {
    let conn = state.db()?;
    db::delete_import(&conn, import_id)
}
