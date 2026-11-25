//! Dátové typy, ktoré putujú medzi parserom, databázou a frontendom.
//!
//! `#[serde(rename_all = "camelCase")]` premenuje polia pri serializácii,
//! takže v Ruste máme `amount_cents` a v TypeScripte `amountCents`.

use chrono::NaiveDate;
use serde::Serialize;

/// Transakcia tak, ako vyšla z parsera – ešte nie je v databáze.
///
/// Sumy držíme v **centoch ako `i64`**, nikdy nie ako `f64`:
/// `0.1 + 0.2 != 0.3` v desatinných číslach s plávajúcou čiarkou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTransaction {
    pub date: NaiveDate,
    pub amount_cents: i64,
    pub currency: String,
    pub counterparty: Option<String>,
    pub counterparty_account: Option<String>,
    pub description: Option<String>,
    pub variable_symbol: Option<String>,
}

/// Riadok, ktorý sa nepodarilo spracovať. Import kvôli nemu nezlyhá –
/// používateľ len uvidí zoznam preskočených riadkov.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowError {
    pub line: usize,
    pub message: String,
}

/// Výsledok importu jedného súboru – posiela sa na frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub file_name: String,
    pub format: String,
    pub imported: usize,
    pub duplicates: usize,
    pub skipped: Vec<RowError>,
}

/// Transakcia uložená v databáze (s id a kategóriou).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: i64,
    pub import_id: i64,
    /// ISO dátum "2026-11-24" – v JS sa dá porovnávať ako reťazec.
    pub date: String,
    pub amount_cents: i64,
    pub currency: String,
    pub counterparty: Option<String>,
    pub counterparty_account: Option<String>,
    pub description: Option<String>,
    pub variable_symbol: Option<String>,
    pub category_id: Option<i64>,
    pub category: Option<String>,
    pub category_manual: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: i64,
    pub pattern: String,
    pub category_id: i64,
    pub category: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    pub id: i64,
    pub file_name: String,
    pub format: String,
    pub imported_at: String,
    pub row_count: i64,
}
