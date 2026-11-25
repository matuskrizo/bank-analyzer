//! CSV export z Georgea (Slovenská sporiteľňa) a podobné CSV výpisy.
//!
//! Neviažeme sa na presné poradie stĺpcov – banky ho z času na čas menia.
//! Namiesto toho nájdeme riadok s hlavičkou a stĺpce identifikujeme
//! podľa názvu (bez diakritiky, malými písmenami). Ak tvoj export používa
//! iný názov stĺpca, stačí ho doplniť do zoznamov aliasov nižšie.

use csv::ReaderBuilder;

use super::{ParseOutput, amount, date, text};
use crate::error::{AppError, AppResult};
use crate::model::{ParsedTransaction, RowError};

// --- Aliasy názvov stĺpcov (už "zložené" cez text::fold) -------------------
// Poradie je dôležité: skôr uvedený alias má prednosť.
// Aliasy s max. 3 znakmi ("vs") musia sedieť presne, dlhšie stačí ako podreťazec.

const DATE: &[&str] = &[
    "datum zauctovania",
    "datum transakcie",
    "datum realizacie",
    "datum spracovania",
    "datum platby",
    "booking date",
    "datum splatnosti", // George (SLSP) exportuje len tento dátum
    "datum",
];
const AMOUNT: &[&str] = &["suma", "ciastka", "amount", "objem", "hodnota"];
const CURRENCY: &[&str] = &["mena", "currency"];
const COUNTERPARTY: &[&str] = &[
    "nazov partnera",
    "nazov protiuctu",
    "nazov protistrany",
    "nazov uctu protistrany",
    "partner",
    "protistrana",
    "prijemca",
    "platitel",
    "obchodnik",
    "counterparty",
];
const ACCOUNT: &[&str] = &[
    "iban partnera",
    "iban protiuctu",
    "iban protistrany",
    "cislo uctu partnera",
    "cislo protiuctu",
    "protiucet",
    "iban",
];
const DESCRIPTION: &[&str] = &[
    "popis",
    "poznamka",
    "sprava pre prijemcu",
    "informacia pre prijemcu",
    "sprava",
    "detail",
    "ucel platby",
    "referencia platitela",
    "description",
    "note",
    "typ transakcie", // "Platba kartou", "Výber kartou", "Poplatok"… – pomáha kategorizácii
];
const VARIABLE_SYMBOL: &[&str] = &["variabilny symbol", "variabilny", "vs"];

/// Indexy stĺpcov, ktoré sme v hlavičke našli.
#[derive(Debug, PartialEq)]
struct ColumnMap {
    date: usize,
    amount: usize,
    currency: Option<usize>,
    counterparty: Option<usize>,
    account: Option<usize>,
    description: Vec<usize>,
    variable_symbol: Option<usize>,
}

impl ColumnMap {
    /// Vráti `None`, ak hlavička nemá aspoň dátum a sumu.
    fn from_headers(headers: &[String]) -> Option<ColumnMap> {
        let date = find_column(headers, DATE, &[])?;
        let amount = find_column(headers, AMOUNT, &["povodn", "poplat", "zostatok"])?;
        let counterparty = find_column(
            headers,
            COUNTERPARTY,
            &["iban", "cislo", "bic", "swift", "kod", "banka"],
        );
        let account = find_column(headers, ACCOUNT, &["bic", "nazov"]);

        // Popis môže byť rozdelený do viacerých stĺpcov – zoberieme všetky.
        let description = (0..headers.len())
            .filter(|&i| DESCRIPTION.iter().any(|a| matches(&headers[i], a)))
            .filter(|&i| Some(i) != counterparty && Some(i) != account)
            .collect();

        Some(ColumnMap {
            date,
            amount,
            currency: find_column(headers, CURRENCY, &["povodn"]),
            counterparty,
            account,
            description,
            variable_symbol: find_column(headers, VARIABLE_SYMBOL, &[]),
        })
    }
}

fn matches(header: &str, alias: &str) -> bool {
    if alias.len() <= 3 {
        header == alias
    } else {
        header.contains(alias)
    }
}

/// Nájde prvý stĺpec, ktorý zodpovedá aliasom. Najprv hľadá presnú zhodu,
/// potom podreťazec. Stĺpce obsahujúce slovo z `forbidden` preskočí
/// (napr. "IBAN partnera" obsahuje "partner", ale nie je to meno).
fn find_column(headers: &[String], aliases: &[&str], forbidden: &[&str]) -> Option<usize> {
    let allowed = |h: &String| !forbidden.iter().any(|f| h.contains(f));
    let exact = aliases
        .iter()
        .find_map(|a| headers.iter().position(|h| allowed(h) && h == a));
    exact.or_else(|| {
        aliases
            .iter()
            .find_map(|a| headers.iter().position(|h| allowed(h) && matches(h, a)))
    })
}

/// Hľadá hlavičku v prvých riadkoch (pred ňou môžu byť info o účte).
/// Vráti (index riadku, bajtový offset riadku, oddeľovač, mapa stĺpcov).
fn find_header(content: &str) -> Option<(usize, usize, u8, ColumnMap)> {
    let mut offset = 0;
    for (index, line) in content.split_inclusive('\n').take(30).enumerate() {
        for &delimiter in b";,\t" {
            let headers: Vec<String> = ReaderBuilder::new()
                .has_headers(false)
                .delimiter(delimiter)
                .from_reader(line.as_bytes())
                .records()
                .next()
                .and_then(Result::ok)
                .map(|record| record.iter().map(text::fold).collect())
                .unwrap_or_default();

            // `if let ... && ...` (let chains) – novinka edície Rust 2024.
            if headers.len() >= 2
                && let Some(map) = ColumnMap::from_headers(&headers)
            {
                return Some((index, offset, delimiter, map));
            }
        }
        offset += line.len();
    }
    None
}

pub fn parse(content: &str) -> AppResult<ParseOutput> {
    let (header_index, offset, delimiter, map) = find_header(content).ok_or_else(|| {
        AppError::UnknownFormat("v CSV som nenašiel hlavičku so stĺpcami dátumu a sumy".to_owned())
    })?;

    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true) // riadky môžu mať rôzny počet stĺpcov
        .from_reader(&content.as_bytes()[offset..]);

    let mut transactions = Vec::new();
    let mut errors = Vec::new();

    for result in reader.records() {
        let record = match result {
            Ok(record) => record,
            Err(err) => {
                let line = err.position().map_or(0, |p| p.line() as usize) + header_index;
                errors.push(RowError {
                    line,
                    message: err.to_string(),
                });
                continue;
            }
        };
        if record.iter().all(|field| field.trim().is_empty()) {
            continue; // prázdny riadok na konci súboru
        }

        // Číslo riadku v pôvodnom súbore (1 = prvý riadok).
        let line = header_index + record.position().map_or(0, |p| p.line() as usize);
        let field = |i: usize| record.get(i).unwrap_or("");

        match parse_row(&map, field) {
            Ok(tx) => transactions.push(tx),
            Err(message) => errors.push(RowError { line, message }),
        }
    }

    Ok(ParseOutput {
        format: "CSV",
        transactions,
        errors,
    })
}

/// Prevedie jeden riadok na transakciu. `field` je closure, ktorá vráti
/// hodnotu stĺpca podľa indexu – vďaka tomu sa funkcia ľahko testuje.
fn parse_row<'a>(
    map: &ColumnMap,
    field: impl Fn(usize) -> &'a str,
) -> Result<ParsedTransaction, String> {
    let date = date::parse_date(field(map.date))?;
    let amount_cents = amount::parse_amount_cents(field(map.amount))?;

    let currency = map
        .currency
        .and_then(|i| text::non_empty(field(i)))
        .unwrap_or_else(|| "EUR".to_owned());

    // Spojíme neprázdne, navzájom rôzne časti popisu.
    let mut parts: Vec<String> = Vec::new();
    for &i in &map.description {
        if let Some(part) = text::non_empty(field(i))
            && !parts.contains(&part)
        {
            parts.push(part);
        }
    }
    let description = (!parts.is_empty()).then(|| parts.join(" | "));

    Ok(ParsedTransaction {
        date,
        amount_cents,
        currency,
        counterparty: map.counterparty.and_then(|i| text::non_empty(field(i))),
        counterparty_account: map.account.and_then(|i| text::non_empty(field(i))),
        description,
        variable_symbol: map.variable_symbol.and_then(|i| text::non_empty(field(i))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    const SAMPLE: &str = include_str!("../../../samples/george_ukazka.csv");

    #[test]
    fn parses_sample_export() {
        let out = parse(SAMPLE).unwrap();
        assert!(out.errors.is_empty(), "chyby: {:?}", out.errors);
        assert_eq!(out.transactions.len(), 24);

        let first = &out.transactions[0];
        assert_eq!(first.date, NaiveDate::from_ymd_opt(2026, 11, 21).unwrap());
        assert_eq!(first.amount_cents, -4731);
        assert_eq!(
            first.counterparty.as_deref(),
            Some("LIDL DAKUJEME ZA NAKUP")
        );
        assert_eq!(first.currency, "EUR");
    }

    /// Hlavička skutočného CSV exportu z Georgea (SLSP), riadky sú vymyslené.
    #[test]
    fn parses_slsp_george_export() {
        let csv = concat!(
            r#""Vlastný názov účtu","Vlastný IBAN","Dátum splatnosti","Suma","Mena","Partner","IBAN partnera","BIC SWIFT kód banky partnera","Číslo účtu partnera","CC kód banky partnera","Popis transakcie","Typ transakcie","Konštantný symbol","Špecifický symbol","Variabilný symbol""#,
            "\n",
            r#""Bežný účet","SK0000000000000000000000","23.11.2026","-19,92","EUR","BILLA 123","","","","","","Platba kartou","","","""#,
            "\n",
            r#""Bežný účet","SK0000000000000000000000","10.11.2026","3 200,00","EUR","Fiktívna Firma","SK1100000000000000000001","","SK1100000000000000000001","","Mzda","Prijatá platba","","","202610""#,
            "\n",
            r#""Bežný účet","SK0000000000000000000000","28.10.2026","-140,00","EUR","","","","","","","Výber kartou","","","""#,
            "\n",
        );
        let out = parse(csv).unwrap();
        assert!(out.errors.is_empty(), "chyby: {:?}", out.errors);
        assert_eq!(out.transactions.len(), 3);

        let card = &out.transactions[0];
        assert_eq!(card.date, NaiveDate::from_ymd_opt(2026, 11, 23).unwrap());
        assert_eq!(card.amount_cents, -1992);
        assert_eq!(card.counterparty.as_deref(), Some("BILLA 123"));
        assert_eq!(card.description.as_deref(), Some("Platba kartou"));

        let salary = &out.transactions[1];
        assert_eq!(salary.amount_cents, 320000);
        assert_eq!(
            salary.counterparty_account.as_deref(),
            Some("SK1100000000000000000001")
        );
        assert_eq!(salary.description.as_deref(), Some("Mzda | Prijatá platba"));
        assert_eq!(salary.variable_symbol.as_deref(), Some("202610"));

        let atm = &out.transactions[2];
        assert_eq!(atm.counterparty, None);
        assert_eq!(atm.description.as_deref(), Some("Výber kartou"));
    }

    #[test]
    fn maps_columns_by_name() {
        let headers: Vec<String> = [
            "Dátum zaúčtovania",
            "Názov partnera",
            "IBAN partnera",
            "Suma",
            "Mena",
            "Variabilný symbol",
            "Popis",
            "Poznámka",
        ]
        .iter()
        .map(|h| text::fold(h))
        .collect();

        let map = ColumnMap::from_headers(&headers).unwrap();
        assert_eq!(map.date, 0);
        assert_eq!(map.counterparty, Some(1)); // nie "IBAN partnera"
        assert_eq!(map.account, Some(2));
        assert_eq!(map.amount, 3);
        assert_eq!(map.currency, Some(4));
        assert_eq!(map.variable_symbol, Some(5));
        assert_eq!(map.description, vec![6, 7]);
    }

    #[test]
    fn handles_preamble_comma_and_bad_rows() {
        let csv = "Výpis z účtu SK00 0000 0000 0000 0000 0000\n\
                   \n\
                   Datum,Protistrana,Suma\n\
                   1.11.2026,Kaviareň,\"-3,50\"\n\
                   nie-dátum,Niečo,\"-1,00\"\n\
                   2.11.2026,Výplata,\"1500,00\"\n";
        let out = parse(csv).unwrap();
        assert_eq!(out.transactions.len(), 2);
        assert_eq!(out.transactions[0].amount_cents, -350);
        assert_eq!(out.transactions[1].amount_cents, 150000);
        assert_eq!(out.errors.len(), 1);
        assert_eq!(out.errors[0].line, 5);
    }

    #[test]
    fn rejects_csv_without_required_columns() {
        let err = parse("Meno;Priezvisko\nJán;Novák\n").unwrap_err();
        assert!(matches!(err, AppError::UnknownFormat(_)));
    }
}
