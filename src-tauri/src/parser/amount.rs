//! Parsovanie súm do centov.
//!
//! Banky píšu sumy rôzne: "-1 234,56", "-1234.56", "1.234,56 EUR", "12,5".
//! Výsledok je vždy celé číslo v centoch (i64), bez `f64`.

/// "-1 234,56 EUR" → `Ok(-123456)`
pub fn parse_amount_cents(raw: &str) -> Result<i64, String> {
    // Necháme len číslice, znamienka a oddeľovače; zahodíme medzery,
    // nezlomiteľné medzery (U+00A0), "EUR" atď. Unicode mínus (U+2212) → '-'.
    let cleaned: String = raw
        .chars()
        .map(|c| if c == '\u{2212}' { '-' } else { c })
        .filter(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | ',' | '.'))
        .collect();

    if !cleaned.chars().any(|c| c.is_ascii_digit()) {
        return Err(format!("'{}' nie je suma", raw.trim()));
    }

    let negative = cleaned.starts_with('-');
    let body = cleaned.trim_start_matches(['-', '+']);

    // Desatinný oddeľovač je POSLEDNÁ čiarka alebo bodka, ak za ňou
    // nasledujú najviac 2 číslice. "1.234" je teda 1234 (tisíce),
    // ale "1.23" je 1,23.
    let (int_part, frac_part) = match body.rfind([',', '.']) {
        Some(pos) if body.len() - pos - 1 <= 2 => (&body[..pos], &body[pos + 1..]),
        _ => (body, ""),
    };

    let int_digits: String = int_part.chars().filter(char::is_ascii_digit).collect();
    let int_value: i64 = if int_digits.is_empty() {
        0
    } else {
        int_digits
            .parse()
            .map_err(|_| format!("'{}' je príliš veľká suma", raw.trim()))?
    };

    // "5" → "50" centov, "" → "00"
    let frac_value: i64 = format!("{frac_part:0<2}")
        .parse()
        .map_err(|_| format!("'{}' nie je platná suma", raw.trim()))?;

    let cents = int_value
        .checked_mul(100)
        .and_then(|v| v.checked_add(frac_value))
        .ok_or_else(|| format!("'{}' je príliš veľká suma", raw.trim()))?;

    Ok(if negative { -cents } else { cents })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_formats() {
        let cases = [
            ("-1 234,56", -123456),
            ("-1234.56", -123456),
            ("1.234,56 EUR", 123456),
            ("1,234.56", 123456),
            ("12,5", 1250),
            ("+20", 2000),
            ("1.234", 123400),
            ("-0,99", -99),
            ("\u{2212}15,00", -1500),
            ("2\u{a0}500,00", 250000),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_amount_cents(input), Ok(expected), "vstup: {input:?}");
        }
    }

    #[test]
    fn rejects_text() {
        assert!(parse_amount_cents("EUR").is_err());
        assert!(parse_amount_cents("").is_err());
    }
}
