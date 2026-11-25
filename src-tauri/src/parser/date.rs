//! Parsovanie dátumov v rôznych formátoch.

use chrono::NaiveDate;

/// Formáty, ktoré skúšame po poradí. Prvý, ktorý sedí, vyhráva.
const FORMATS: [&str; 3] = ["%d.%m.%Y", "%Y-%m-%d", "%d/%m/%Y"];

/// "24.11.2026", "24. 11. 2026", "2026-11-24T10:22:00" → NaiveDate
pub fn parse_date(raw: &str) -> Result<NaiveDate, String> {
    let normalized = raw.trim().replace(". ", ".");
    for fmt in FORMATS {
        // `parse_and_remainder` prečíta dátum zo začiatku a zvyšok
        // (napr. čas "10:22") vráti bokom – ten nás nezaujíma.
        if let Ok((date, _rest)) = NaiveDate::parse_and_remainder(&normalized, fmt) {
            return Ok(date);
        }
    }
    Err(format!("'{}' nie je dátum", raw.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn parses_formats() {
        assert_eq!(parse_date("24.11.2026"), Ok(d(2026, 11, 24)));
        assert_eq!(parse_date("24. 11. 2026"), Ok(d(2026, 11, 24)));
        assert_eq!(parse_date("2026-11-24"), Ok(d(2026, 11, 24)));
        assert_eq!(parse_date("2026-11-24T10:22:00+01:00"), Ok(d(2026, 11, 24)));
        assert_eq!(parse_date("24.11.2026 10:22"), Ok(d(2026, 11, 24)));
        assert_eq!(parse_date("1.3.2026"), Ok(d(2026, 3, 1)));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_date("včera").is_err());
        assert!(parse_date("32.13.2026").is_err());
    }
}
