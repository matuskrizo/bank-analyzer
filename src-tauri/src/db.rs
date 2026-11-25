//! Lokálna SQLite databáza cez `rusqlite`.
//!
//! - Schéma sa vytvára **migráciami** podľa `PRAGMA user_version`:
//!   každá migrácia sa spustí len raz a číslo verzie sa uloží do súboru DB.
//! - Duplicity pri opakovanom importe rieši stĺpec `hash` s `UNIQUE`
//!   a príkaz `INSERT OR IGNORE`.
//! - Všetky funkcie berú `&Connection` / `&mut Connection`, nie globálny stav,
//!   takže sa dajú testovať s databázou v pamäti.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, Transaction as SqlTx, params};
use sha2::{Digest, Sha256};

use crate::categorize::{self, MatchRule};
use crate::error::{AppError, AppResult};
use crate::model::{Category, ImportInfo, ImportReport, ParsedTransaction, Rule, Transaction};
use crate::parser::ParseOutput;

// ---------------------------------------------------------------------------
// Otvorenie a migrácie
// ---------------------------------------------------------------------------

pub fn open(path: &Path) -> AppResult<Connection> {
    let mut conn = Connection::open(path)?;
    init(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    init(&mut conn)?;
    Ok(conn)
}

fn init(conn: &mut Connection) -> AppResult<()> {
    // Cudzie kľúče sú v SQLite štandardne vypnuté – bez tohto by
    // ON DELETE CASCADE nefungovalo.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)
}

/// Zoznam migrácií. Nikdy needituj existujúcu – pridaj novú na koniec.
const MIGRATIONS: &[fn(&SqlTx) -> rusqlite::Result<()>] =
    &[migration_1_schema, migration_2_more_rules];

fn migrate(conn: &mut Connection) -> AppResult<()> {
    // SQLite pozná len i64 – rusqlite preto usize priamo nepodporuje.
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        // Každá migrácia beží v transakcii: buď prebehne celá, alebo vôbec.
        let tx = conn.transaction()?;
        migration(&tx)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

fn migration_1_schema(tx: &SqlTx) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE imports (
            id          INTEGER PRIMARY KEY,
            file_name   TEXT NOT NULL,
            format      TEXT NOT NULL,
            imported_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            row_count   INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE categories (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );
        CREATE TABLE rules (
            id          INTEGER PRIMARY KEY,
            pattern     TEXT NOT NULL UNIQUE,
            category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE
        );
        CREATE TABLE transactions (
            id                   INTEGER PRIMARY KEY,
            import_id            INTEGER NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
            date                 TEXT NOT NULL,      -- ISO 8601: 2026-11-24
            amount_cents         INTEGER NOT NULL,   -- záporné = výdavok
            currency             TEXT NOT NULL,
            counterparty         TEXT,
            counterparty_account TEXT,
            description          TEXT,
            variable_symbol      TEXT,
            category_id          INTEGER REFERENCES categories(id) ON DELETE SET NULL,
            category_manual      INTEGER NOT NULL DEFAULT 0, -- 1 = nastavil používateľ
            hash                 TEXT NOT NULL UNIQUE
        );
        CREATE INDEX idx_transactions_date ON transactions(date);",
    )?;
    seed_defaults(tx)
}

/// Predvolené kategórie a pravidlá pre bežných slovenských obchodníkov.
const DEFAULT_RULES: &[(&str, &[&str])] = &[
    (
        "Potraviny",
        &[
            "LIDL",
            "TESCO",
            "BILLA",
            "KAUFLAND",
            "COOP JEDNOTA",
            "KRAJ MARKET",
            "FRESH",
        ],
    ),
    (
        "Reštaurácie a jedlo",
        &[
            "BOLT FOOD",
            "WOLT",
            "FOODORA",
            "MCDONALD",
            "KFC",
            "BURGER KING",
            "STARBUCKS",
        ],
    ),
    (
        "Doprava",
        &["BOLT", "UBER", "ZSSK", "ARRIVA", "SLOVAK LINES", "DPB"],
    ),
    ("Palivo", &["SHELL", "OMV", "SLOVNAFT"]),
    ("Bývanie", &["NAJOM", "SPRAVA BYTOV"]),
    ("Energie", &["ZSE", "SPP", "VODAREN"]),
    (
        "Telefón a internet",
        &["ORANGE", "TELEKOM", "O2 SLOVAKIA", "4KA"],
    ),
    (
        "Predplatné",
        &[
            "NETFLIX",
            "SPOTIFY",
            "HBO",
            "DISNEY",
            "YOUTUBE",
            "APPLE.COM",
        ],
    ),
    ("Drogéria", &["DM DROGERIE", "TETA DROGERIE", "ROSSMANN"]),
    ("Hotovosť", &["BANKOMAT", "VYBER HOTOVOSTI"]),
    ("Príjem", &["MZDA", "VYPLATA"]),
];

fn seed_defaults(tx: &SqlTx) -> rusqlite::Result<()> {
    for (category, patterns) in DEFAULT_RULES {
        tx.execute("INSERT INTO categories (name) VALUES (?1)", [category])?;
        let category_id = tx.last_insert_rowid();
        for pattern in *patterns {
            tx.execute(
                "INSERT INTO rules (pattern, category_id) VALUES (?1, ?2)",
                params![pattern.trim(), category_id],
            )?;
        }
    }
    Ok(())
}

/// Migrácia 2: pravidlá pre typy transakcií zo SLSP a ďalších obchodníkov.
///
/// Prečo nová migrácia a nie úprava `DEFAULT_RULES`? Migrácia 1 už na tvojom
/// počítači prebehla (databáza má `user_version = 1`), takže zmena v nej by sa
/// nikdy nespustila. Nová migrácia sa spustí pri ďalšom štarte aplikácie.
/// `INSERT OR IGNORE` zaistí, že existujúce kategórie a vzory sa nezdvoja.
const MORE_RULES: &[(&str, &[&str])] = &[
    ("Potraviny", &["COOP", "KOSIK.SK", "POTRAVINY"]),
    (
        "Reštaurácie a jedlo",
        &["BISTRO", "PIZZ", "SUSHI", "RESTAURA"],
    ),
    ("Doprava", &["PARKING", "GARAGE", "AUTOCESTA", "EMATRICA"]),
    ("Palivo", &["DALIOIL", "PETROL", "TANKER", "CS PHM"]),
    (
        "Energie",
        &[
            "STREDOSLOVENSKA ENERGETIKA",
            "ENERGETIKA SLOVENSKO",
            "PLYNAREN",
            "VODAR",
            "VOD.PREV",
        ],
    ),
    ("Zdravie a lekáreň", &["DRMAX", "DR.MAX", "BENU", "LEKAREN"]),
    ("Bývanie", &["BYTOVY DOM"]),
    (
        "Domácnosť",
        &["JYSK", "MERKURY", "IKEA", "HORNBACH", "XXXLUTZ"],
    ),
    ("Elektronika", &["ALZA", "PLANEO", "DATART"]),
    ("Šport", &["DECATHLON", "SPORTISIMO"]),
    ("Oblečenie", &["SINSAY", "PEPCO", "NEW YORKER"]),
    ("Zvieratá", &["ZOOPLUS"]),
    (
        "Poistenie",
        &["KOOPERATIVA", "VSZP", "ALLIANZ", "GENERALI", "DOVERA"],
    ),
    ("Investície", &["FINANCNE NASTROJE", "XTB"]),
    ("Poplatky", &["POPLATOK", "POPLATOK ZA VYBER"]), // dlhší vzor vyhrá nad "VYBER KARTOU"
    ("Úvery", &["UVERY"]),
    ("Hotovosť", &["VYBER KARTOU", "VYBER Z BANKOMATU"]),
    ("Predplatné", &["DENNIKN", "SKYSHOWTIME"]),
];

fn migration_2_more_rules(tx: &SqlTx) -> rusqlite::Result<()> {
    for (category, patterns) in MORE_RULES {
        tx.execute(
            "INSERT OR IGNORE INTO categories (name) VALUES (?1)",
            [category],
        )?;
        let category_id: i64 = tx.query_row(
            "SELECT id FROM categories WHERE name = ?1",
            [category],
            |r| r.get(0),
        )?;
        for pattern in *patterns {
            tx.execute(
                "INSERT OR IGNORE INTO rules (pattern, category_id) VALUES (?1, ?2)",
                params![pattern.trim(), category_id],
            )?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// Odtlačok transakcie. `occurrence` rozlišuje úplne rovnaké platby
/// v jednom súbore (dve kávy za 2,50 € v ten istý deň).
fn transaction_hash(t: &ParsedTransaction, occurrence: u32) -> String {
    let key = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        t.date,
        t.amount_cents,
        t.currency,
        t.counterparty.as_deref().unwrap_or(""),
        t.description.as_deref().unwrap_or(""),
        t.variable_symbol.as_deref().unwrap_or(""),
        occurrence
    );
    // Prečo nie std `DefaultHasher`? Jeho výstup sa môže zmeniť medzi
    // verziami Rustu – a hash ukladáme natrvalo do databázy.
    Sha256::digest(key.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn import(
    conn: &mut Connection,
    file_name: &str,
    parsed: ParseOutput,
) -> AppResult<ImportReport> {
    let rules = load_match_rules(conn)?;

    // Celý import v jednej transakcii – je to rýchlejšie (jeden zápis na disk)
    // a pri chybe sa neuloží polovica súboru.
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO imports (file_name, format) VALUES (?1, ?2)",
        params![file_name, parsed.format],
    )?;
    let import_id = tx.last_insert_rowid();

    let mut imported = 0;
    let mut occurrences: HashMap<String, u32> = HashMap::new();
    {
        // `stmt` si požičiava `tx`, preto je v samostatnom bloku –
        // musí zaniknúť skôr, ako zavoláme `tx.commit()`.
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO transactions
                (import_id, date, amount_cents, currency, counterparty,
                 counterparty_account, description, variable_symbol, category_id, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;

        for t in &parsed.transactions {
            let base_hash = transaction_hash(t, 0);
            let occurrence = occurrences.entry(base_hash).or_insert(0);
            let hash = transaction_hash(t, *occurrence);
            *occurrence += 1;

            let text =
                categorize::searchable_text(t.counterparty.as_deref(), t.description.as_deref());
            let category_id = categorize::categorize(&text, &rules);

            // `execute` vráti počet zmenených riadkov: 0 = duplicita (IGNORE).
            imported += stmt.execute(params![
                import_id,
                t.date.to_string(),
                t.amount_cents,
                t.currency,
                t.counterparty,
                t.counterparty_account,
                t.description,
                t.variable_symbol,
                category_id,
                hash,
            ])?;
        }
    }

    if imported == 0 {
        // Nič nové – záznam o importe nechceme.
        tx.execute("DELETE FROM imports WHERE id = ?1", [import_id])?;
    } else {
        tx.execute(
            "UPDATE imports SET row_count = ?1 WHERE id = ?2",
            params![imported as i64, import_id],
        )?;
    }
    tx.commit()?;

    Ok(ImportReport {
        file_name: file_name.to_owned(),
        format: parsed.format.to_owned(),
        imported,
        duplicates: parsed.transactions.len() - imported,
        skipped: parsed.errors,
    })
}

// ---------------------------------------------------------------------------
// Čítanie
// ---------------------------------------------------------------------------

pub fn list_transactions(conn: &Connection) -> AppResult<Vec<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.import_id, t.date, t.amount_cents, t.currency, t.counterparty,
                t.counterparty_account, t.description, t.variable_symbol,
                t.category_id, c.name, t.category_manual
         FROM transactions t
         LEFT JOIN categories c ON c.id = t.category_id
         ORDER BY t.date DESC, t.id DESC",
    )?;
    // `query_map` vráti iterátor Result-ov; `collect` ich zloží do
    // jedného `Result<Vec<_>, _>` – zlyhá pri prvej chybe.
    let rows = stmt.query_map([], |row| {
        Ok(Transaction {
            id: row.get(0)?,
            import_id: row.get(1)?,
            date: row.get(2)?,
            amount_cents: row.get(3)?,
            currency: row.get(4)?,
            counterparty: row.get(5)?,
            counterparty_account: row.get(6)?,
            description: row.get(7)?,
            variable_symbol: row.get(8)?,
            category_id: row.get(9)?,
            category: row.get(10)?,
            category_manual: row.get(11)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_categories(conn: &Connection) -> AppResult<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT id, name FROM categories ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_rules(conn: &Connection) -> AppResult<Vec<Rule>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.pattern, r.category_id, c.name
         FROM rules r JOIN categories c ON c.id = r.category_id
         ORDER BY c.name, r.pattern",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Rule {
            id: row.get(0)?,
            pattern: row.get(1)?,
            category_id: row.get(2)?,
            category: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_imports(conn: &Connection) -> AppResult<Vec<ImportInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_name, format, imported_at, row_count FROM imports ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ImportInfo {
            id: row.get(0)?,
            file_name: row.get(1)?,
            format: row.get(2)?,
            imported_at: row.get(3)?,
            row_count: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_match_rules(conn: &Connection) -> AppResult<Vec<MatchRule>> {
    let mut stmt = conn.prepare("SELECT pattern, category_id FROM rules")?;
    let rows = stmt.query_map([], |row| {
        let pattern: String = row.get(0)?;
        Ok(MatchRule::new(&pattern, row.get(1)?))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---------------------------------------------------------------------------
// Zápis
// ---------------------------------------------------------------------------

pub fn add_category(conn: &Connection, name: &str) -> AppResult<Category> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("názov kategórie je prázdny".into()));
    }
    conn.execute(
        "INSERT OR IGNORE INTO categories (name) VALUES (?1)",
        [name],
    )?;
    let id = conn.query_row("SELECT id FROM categories WHERE name = ?1", [name], |r| {
        r.get(0)
    })?;
    Ok(Category {
        id,
        name: name.to_owned(),
    })
}

/// Pridá pravidlo, alebo zmení kategóriu existujúceho vzoru.
pub fn upsert_rule(conn: &Connection, pattern: &str, category_id: i64) -> AppResult<()> {
    let pattern = pattern.trim();
    if pattern.chars().count() < 2 {
        return Err(AppError::InvalidInput("vzor musí mať aspoň 2 znaky".into()));
    }
    conn.execute(
        "INSERT INTO rules (pattern, category_id) VALUES (?1, ?2)
         ON CONFLICT(pattern) DO UPDATE SET category_id = excluded.category_id",
        params![pattern, category_id],
    )?;
    Ok(())
}

pub fn delete_rule(conn: &Connection, rule_id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM rules WHERE id = ?1", [rule_id])?;
    Ok(())
}

/// Ručné nastavenie kategórie. Takúto transakciu už automatika neprepíše.
/// `None` = zrušiť ručné zaradenie a vrátiť transakciu automatike.
pub fn set_category(
    conn: &Connection,
    transaction_id: i64,
    category_id: Option<i64>,
) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE transactions SET category_id = ?1, category_manual = ?2 WHERE id = ?3",
        params![category_id, category_id.is_some(), transaction_id],
    )?;
    if changed == 0 {
        return Err(AppError::InvalidInput(format!(
            "transakcia {transaction_id} neexistuje"
        )));
    }
    Ok(())
}

/// Znovu prejde všetky automaticky zaradené transakcie podľa aktuálnych
/// pravidiel. Vráti počet transakcií, ktorým sa kategória zmenila.
pub fn recategorize(conn: &mut Connection) -> AppResult<usize> {
    let rules = load_match_rules(conn)?;
    let tx = conn.transaction()?;
    let mut changed = 0;
    {
        let mut select = tx.prepare(
            "SELECT id, counterparty, description, category_id
             FROM transactions WHERE category_manual = 0",
        )?;
        let rows = select
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut update = tx.prepare("UPDATE transactions SET category_id = ?1 WHERE id = ?2")?;
        for (id, counterparty, description, current) in rows {
            let text = categorize::searchable_text(counterparty.as_deref(), description.as_deref());
            let new = categorize::categorize(&text, &rules);
            if new != current {
                update.execute(params![new, id])?;
                changed += 1;
            }
        }
    }
    tx.commit()?;
    Ok(changed)
}

/// Zmaže import aj všetky jeho transakcie (ON DELETE CASCADE).
pub fn delete_import(conn: &Connection, import_id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM imports WHERE id = ?1", [import_id])?;
    Ok(())
}

/// Id kategórie podľa názvu – hodí sa v testoch.
#[cfg(test)]
fn category_id(conn: &Connection, name: &str) -> Option<i64> {
    use rusqlite::OptionalExtension;
    conn.query_row("SELECT id FROM categories WHERE name = ?1", [name], |r| {
        r.get(0)
    })
    .optional()
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn sample() -> ParseOutput {
        parser::parse_bytes(include_bytes!("../../samples/george_ukazka.csv")).unwrap()
    }

    #[test]
    fn migration_seeds_categories_and_is_idempotent() {
        let mut conn = open_in_memory().unwrap();
        let count = list_categories(&conn).unwrap().len();
        let new_in_v2 = MORE_RULES
            .iter()
            .filter(|(name, _)| !DEFAULT_RULES.iter().any(|(n, _)| n == name))
            .count();
        assert_eq!(count, DEFAULT_RULES.len() + new_in_v2);
        // Druhé spustenie migrácií nesmie nič zmeniť.
        migrate(&mut conn).unwrap();
        assert_eq!(list_categories(&conn).unwrap().len(), count);
    }

    #[test]
    fn import_twice_detects_duplicates() {
        let mut conn = open_in_memory().unwrap();
        let first = import(&mut conn, "vypis.csv", sample()).unwrap();
        assert_eq!(first.imported, 24);
        assert_eq!(first.duplicates, 0);

        let second = import(&mut conn, "vypis.csv", sample()).unwrap();
        assert_eq!(second.imported, 0);
        assert_eq!(second.duplicates, 24);
        assert_eq!(list_transactions(&conn).unwrap().len(), 24);
        assert_eq!(
            list_imports(&conn).unwrap().len(),
            1,
            "prázdny import sa nemá uložiť"
        );
    }

    #[test]
    fn identical_rows_in_one_file_are_both_kept() {
        let mut conn = open_in_memory().unwrap();
        let csv = "Datum;Partner;Suma\n1.11.2026;Kaviareň;-2,50\n1.11.2026;Kaviareň;-2,50\n";
        let report = import(
            &mut conn,
            "kava.csv",
            parser::parse_bytes(csv.as_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.imported, 2);
    }

    #[test]
    fn categorizes_on_import() {
        let mut conn = open_in_memory().unwrap();
        import(&mut conn, "vypis.csv", sample()).unwrap();
        let txs = list_transactions(&conn).unwrap();
        let find = |cp: &str| {
            txs.iter()
                .find(|t| t.counterparty.as_deref() == Some(cp))
                .unwrap()
        };

        assert_eq!(
            find("LIDL DAKUJEME ZA NAKUP").category.as_deref(),
            Some("Potraviny")
        );
        assert_eq!(
            find("BOLT FOOD").category.as_deref(),
            Some("Reštaurácie a jedlo")
        );
        assert_eq!(find("BOLT.EU/O/2611").category.as_deref(), Some("Doprava"));
        assert_eq!(find("KAVIAREN U ZELENEHO STROMU").category, None);
    }

    #[test]
    fn new_rule_and_manual_category() {
        let mut conn = open_in_memory().unwrap();
        import(&mut conn, "vypis.csv", sample()).unwrap();
        let restaurants = category_id(&conn, "Reštaurácie a jedlo").unwrap();

        upsert_rule(&conn, "kaviaren", restaurants).unwrap();
        assert_eq!(recategorize(&mut conn).unwrap(), 1);

        // Ručne preradená transakcia ostane aj po recategorize.
        let txs = list_transactions(&conn).unwrap();
        let lidl = txs
            .iter()
            .find(|t| t.counterparty.as_deref() == Some("LIDL DAKUJEME ZA NAKUP"))
            .unwrap();
        set_category(&conn, lidl.id, Some(restaurants)).unwrap();
        recategorize(&mut conn).unwrap();
        let txs = list_transactions(&conn).unwrap();
        let lidl = txs.iter().find(|t| t.id == lidl.id).unwrap();
        assert_eq!(lidl.category_id, Some(restaurants));
        assert!(lidl.category_manual);
    }

    #[test]
    fn delete_import_cascades() {
        let mut conn = open_in_memory().unwrap();
        import(&mut conn, "vypis.csv", sample()).unwrap();
        let import_id = list_imports(&conn).unwrap()[0].id;
        delete_import(&conn, import_id).unwrap();
        assert!(list_transactions(&conn).unwrap().is_empty());
    }
}
