//! Parsovanie bankových výpisov.
//!
//! Tento modul **nezávisí od Tauri ani od databázy** – dostane bajty
//! a vráti zoznam transakcií. Preto sa dá celý otestovať cez `cargo test`
//! bez spúšťania aplikácie.
//!
//! Podporované formáty:
//! - CSV export z Georgea (SLSP) a podobné CSV (stĺpce sa hľadajú podľa názvu),
//! - XML výpis ISO 20022 camt.053 (štandard SEPA bánk).

pub mod amount;
pub mod camt053;
pub mod date;
pub mod george_csv;
pub mod text;

use std::path::Path;

use crate::error::AppResult;
use crate::model::{ParsedTransaction, RowError};

/// Výstup parsera: úspešné transakcie + riadky, ktoré sa nepodarilo spracovať.
#[derive(Debug)]
pub struct ParseOutput {
    pub format: &'static str,
    pub transactions: Vec<ParsedTransaction>,
    pub errors: Vec<RowError>,
}

/// Prečíta súbor z disku a rozparsuje ho.
pub fn parse_file(path: &Path) -> AppResult<ParseOutput> {
    // `?` – ak čítanie zlyhá, std::io::Error sa cez `From` zmení na AppError::Io.
    let bytes = std::fs::read(path)?;
    parse_bytes(&bytes)
}

/// Rozpozná formát podľa obsahu (nie podľa prípony – tá môže klamať).
pub fn parse_bytes(bytes: &[u8]) -> AppResult<ParseOutput> {
    let content = text::decode(bytes);
    if content.trim_start().starts_with('<') {
        camt053::parse(&content)
    } else {
        george_csv::parse(&content)
    }
}
