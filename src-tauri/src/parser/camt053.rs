//! XML výpis vo formáte ISO 20022 camt.053 (Bank to Customer Statement).
//!
//! Štruktúru XML popíšeme Rust structmi a `quick-xml` + `serde` ich
//! naplnia samy. Elementy, ktoré nepotrebujeme, serde jednoducho ignoruje.
//!
//! Zjednodušená štruktúra súboru:
//! ```text
//! Document
//! └─ BkToCstmrStmt
//!    └─ Stmt (1..n)
//!       └─ Ntry (0..n)            ← jedna transakcia
//!          ├─ Amt Ccy="EUR"       ← suma (vždy kladná)
//!          ├─ CdtDbtInd           ← CRDT = príjem, DBIT = výdavok
//!          ├─ BookgDt/Dt          ← dátum zaúčtovania
//!          └─ NtryDtls/TxDtls
//!             ├─ Refs/EndToEndId  ← často "/VS123/SS/KS0308"
//!             ├─ RltdPties        ← protistrana (Dbtr/Cdtr) a jej účet
//!             └─ RmtInf/Ustrd     ← správa pre prijímateľa
//! ```

use serde::Deserialize;

use super::{ParseOutput, amount, date, text};
use crate::error::AppResult;
use crate::model::{ParsedTransaction, RowError};

// `rename` mapuje XML názvy elementov (PascalCase skratky) na Rust polia.
// `default` znamená: ak element chýba, použi prázdnu hodnotu (Vec::new(), None).

#[derive(Debug, Deserialize)]
struct Document {
    #[serde(rename = "BkToCstmrStmt")]
    statement_root: StatementRoot,
}

#[derive(Debug, Deserialize)]
struct StatementRoot {
    #[serde(rename = "Stmt", default)]
    statements: Vec<Statement>,
}

#[derive(Debug, Deserialize)]
struct Statement {
    #[serde(rename = "Ntry", default)]
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(rename = "Amt")]
    amount: Amount,
    #[serde(rename = "CdtDbtInd")]
    credit_debit: String,
    #[serde(rename = "BookgDt")]
    booking_date: Option<DateChoice>,
    #[serde(rename = "ValDt")]
    value_date: Option<DateChoice>,
    #[serde(rename = "NtryDtls", default)]
    details: Vec<EntryDetails>,
    #[serde(rename = "AddtlNtryInf")]
    additional_info: Option<String>,
}

/// `<Amt Ccy="EUR">12.50</Amt>` – atribút má prefix `@`, text uzla je `$text`.
#[derive(Debug, Deserialize)]
struct Amount {
    #[serde(rename = "@Ccy")]
    currency: String,
    #[serde(rename = "$text")]
    value: String,
}

/// Dátum môže byť `<Dt>2026-11-24</Dt>` alebo `<DtTm>2026-11-24T10:00:00</DtTm>`.
#[derive(Debug, Deserialize)]
struct DateChoice {
    #[serde(rename = "Dt")]
    date: Option<String>,
    #[serde(rename = "DtTm")]
    date_time: Option<String>,
}

impl DateChoice {
    fn as_str(&self) -> Option<&str> {
        self.date.as_deref().or(self.date_time.as_deref())
    }
}

#[derive(Debug, Deserialize)]
struct EntryDetails {
    #[serde(rename = "TxDtls", default)]
    transactions: Vec<TxDetails>,
}

#[derive(Debug, Deserialize)]
struct TxDetails {
    #[serde(rename = "Refs")]
    refs: Option<Refs>,
    #[serde(rename = "RltdPties")]
    parties: Option<RelatedParties>,
    #[serde(rename = "RmtInf")]
    remittance: Option<Remittance>,
    #[serde(rename = "AddtlTxInf")]
    additional_info: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Refs {
    #[serde(rename = "EndToEndId")]
    end_to_end_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RelatedParties {
    #[serde(rename = "Dbtr")]
    debtor: Option<Party>,
    #[serde(rename = "DbtrAcct")]
    debtor_account: Option<Account>,
    #[serde(rename = "Cdtr")]
    creditor: Option<Party>,
    #[serde(rename = "CdtrAcct")]
    creditor_account: Option<Account>,
}

/// Staršie verzie: `<Cdtr><Nm>..</Nm></Cdtr>`,
/// novšie (camt.053.001.08+): `<Cdtr><Pty><Nm>..</Nm></Pty></Cdtr>`.
#[derive(Debug, Deserialize)]
struct Party {
    #[serde(rename = "Nm")]
    name: Option<String>,
    #[serde(rename = "Pty")]
    party: Option<Box<Party>>,
}

impl Party {
    fn name(&self) -> Option<String> {
        self.name
            .as_deref()
            .and_then(text::non_empty)
            .or_else(|| self.party.as_ref().and_then(|p| p.name()))
    }
}

#[derive(Debug, Deserialize)]
struct Account {
    #[serde(rename = "Id")]
    id: AccountId,
}

#[derive(Debug, Deserialize)]
struct AccountId {
    #[serde(rename = "IBAN")]
    iban: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Remittance {
    #[serde(rename = "Ustrd", default)]
    unstructured: Vec<String>,
}

pub fn parse(content: &str) -> AppResult<ParseOutput> {
    // Celé XML naraz → Document. Chyba sa cez `?` zmení na AppError::Xml.
    let document: Document = quick_xml::de::from_str(content)?;

    let mut transactions = Vec::new();
    let mut errors = Vec::new();

    let entries = document
        .statement_root
        .statements
        .iter()
        .flat_map(|statement| &statement.entries);

    for (index, entry) in entries.enumerate() {
        match convert_entry(entry) {
            Ok(tx) => transactions.push(tx),
            // Pri XML nemáme čísla riadkov, tak hlásime poradie záznamu.
            Err(message) => errors.push(RowError {
                line: index + 1,
                message,
            }),
        }
    }

    Ok(ParseOutput {
        format: "camt.053 XML",
        transactions,
        errors,
    })
}

fn convert_entry(entry: &Entry) -> Result<ParsedTransaction, String> {
    let raw_date = entry
        .booking_date
        .as_ref()
        .or(entry.value_date.as_ref())
        .and_then(DateChoice::as_str)
        .ok_or("záznam nemá dátum")?;
    let date = date::parse_date(raw_date)?;

    let abs_cents = amount::parse_amount_cents(&entry.amount.value)?;
    let is_debit = entry.credit_debit.trim() == "DBIT";
    let amount_cents = if is_debit {
        -abs_cents.abs()
    } else {
        abs_cents.abs()
    };

    // Pri dávkových platbách môže byť viac TxDtls – berieme prvý.
    let tx = entry.details.iter().flat_map(|d| &d.transactions).next();
    let parties = tx.and_then(|t| t.parties.as_ref());

    // Pri výdavku je protistrana príjemca (Cdtr), pri príjme platiteľ (Dbtr).
    let (party, account) = match parties {
        Some(p) if is_debit => (p.creditor.as_ref(), p.creditor_account.as_ref()),
        Some(p) => (p.debtor.as_ref(), p.debtor_account.as_ref()),
        None => (None, None),
    };

    let description = tx
        .and_then(|t| t.remittance.as_ref())
        .map(|r| r.unstructured.join(" "))
        .and_then(|s| text::non_empty(&s))
        .or_else(|| {
            tx.and_then(|t| t.additional_info.as_deref())
                .and_then(text::non_empty)
        })
        .or_else(|| entry.additional_info.as_deref().and_then(text::non_empty));

    let variable_symbol = tx
        .and_then(|t| t.refs.as_ref())
        .and_then(|r| r.end_to_end_id.as_deref())
        .and_then(extract_variable_symbol);

    Ok(ParsedTransaction {
        date,
        amount_cents,
        currency: entry.amount.currency.clone(),
        counterparty: party.and_then(Party::name),
        counterparty_account: account
            .and_then(|a| a.id.iban.as_deref())
            .and_then(text::non_empty),
        description,
        variable_symbol,
    })
}

/// "/VS2026001/SS/KS0308" → Some("2026001")
fn extract_variable_symbol(end_to_end_id: &str) -> Option<String> {
    let rest = end_to_end_id.split("/VS").nth(1)?;
    let vs: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (!vs.is_empty()).then_some(vs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../samples/camt053_ukazka.xml");

    #[test]
    fn parses_sample_statement() {
        let out = parse(SAMPLE).unwrap();
        assert!(out.errors.is_empty(), "chyby: {:?}", out.errors);
        assert_eq!(out.transactions.len(), 4);

        let rent = &out.transactions[0];
        assert_eq!(rent.amount_cents, -65000);
        assert_eq!(
            rent.counterparty.as_deref(),
            Some("Správa bytov Horská s.r.o.")
        );
        assert_eq!(rent.variable_symbol.as_deref(), Some("2026011"));
        assert_eq!(rent.description.as_deref(), Some("Nájom 12/2026"));

        let salary = &out.transactions[1];
        assert_eq!(salary.amount_cents, 185000);
        // novší formát s <Pty><Nm>
        assert_eq!(
            salary.counterparty.as_deref(),
            Some("Fiktívna Firma s.r.o.")
        );
    }

    #[test]
    fn extracts_vs() {
        assert_eq!(
            extract_variable_symbol("/VS123/SS/KS0308").as_deref(),
            Some("123")
        );
        assert_eq!(extract_variable_symbol("NOTPROVIDED"), None);
    }
}
