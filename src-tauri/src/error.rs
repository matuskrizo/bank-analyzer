//! Chybové stavy aplikácie.
//!
//! Tauri príkaz môže vrátiť `Result<T, E>`, ale `E` musí implementovať
//! `serde::Serialize`, aby sa dal poslať do JavaScriptu. `thiserror` nám
//! vygeneruje `Display` a `From`, no `Serialize` musíme napísať ručne,
//! pretože vnútorné chyby (napr. `std::io::Error`) serializovať nevedia.
//!
//! Na frontend posielame objekt `{ kind: "io", message: "..." }`, takže
//! v Svelte môžeme reagovať podľa `kind` a zobraziť ľudskú správu.

use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// `#[from]` vygeneruje `impl From<std::io::Error> for AppError`,
    /// vďaka čomu funguje operátor `?` na `std::fs::read(...)`.
    #[error("Súbor sa nepodarilo prečítať: {0}")]
    Io(#[from] std::io::Error),

    #[error("Chyba v CSV: {0}")]
    Csv(#[from] csv::Error),

    #[error("Chyba v XML výpise: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("Chyba databázy: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Nepodporovaný formát výpisu: {0}")]
    UnknownFormat(String),

    #[error("Neplatný vstup: {0}")]
    InvalidInput(String),

    /// Mutex je "otrávený", ak niektoré vlákno spanikárilo, kým ho držalo.
    #[error("Interná chyba: databáza je zablokovaná")]
    LockPoisoned,
}

impl AppError {
    /// Krátky strojový kód chyby – na frontende podľa neho môžeme vetviť.
    fn kind(&self) -> &'static str {
        match self {
            AppError::Io(_) => "io",
            AppError::Csv(_) => "csv",
            AppError::Xml(_) => "xml",
            AppError::Db(_) => "db",
            AppError::UnknownFormat(_) => "format",
            AppError::InvalidInput(_) => "input",
            AppError::LockPoisoned => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

/// Skratka, aby sme nemuseli všade písať `Result<T, AppError>`.
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_kind_and_message() {
        let err = AppError::UnknownFormat("chýba stĺpec Suma".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"format","message":"Nepodporovaný formát výpisu: chýba stĺpec Suma"}"#
        );
    }
}
