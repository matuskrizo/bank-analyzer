//! Pomocné funkcie pre text: kódovanie a porovnávanie bez diakritiky.

use encoding_rs::WINDOWS_1250;

/// Prevedie bajty súboru na `String`.
///
/// Slovenské banky často exportujú vo Windows-1250 (staré kódovanie
/// pre strednú Európu). Najprv skúsime UTF-8; ak bajty nie sú platné
/// UTF-8, dekódujeme ich ako Windows-1250.
pub fn decode(bytes: &[u8]) -> String {
    // UTF-8 BOM (EF BB BF) na začiatku súboru – Excel ho rád pridáva.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);

    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => {
            // `decode` vracia (Cow<str>, použité kódovanie, či boli chyby).
            let (text, _, _) = WINDOWS_1250.decode(bytes);
            text.into_owned()
        }
    }
}

/// Zjednoduší text na porovnávanie: malé písmená, bez diakritiky,
/// zlúčené medzery. "Dátum  zaúčtovania" → "datum zauctovania".
pub fn fold(s: &str) -> String {
    let mapped: String = s
        .chars()
        .map(|c| match c {
            'á' | 'ä' | 'Á' | 'Ä' => 'a',
            'č' | 'Č' => 'c',
            'ď' | 'Ď' => 'd',
            'é' | 'ě' | 'É' | 'Ě' => 'e',
            'í' | 'Í' => 'i',
            'ĺ' | 'ľ' | 'Ĺ' | 'Ľ' => 'l',
            'ň' | 'Ň' => 'n',
            'ó' | 'ô' | 'ö' | 'Ó' | 'Ô' | 'Ö' => 'o',
            'ŕ' | 'ř' | 'Ŕ' | 'Ř' => 'r',
            'š' | 'Š' => 's',
            'ť' | 'Ť' => 't',
            'ú' | 'ů' | 'ü' | 'Ú' | 'Ů' | 'Ü' => 'u',
            'ý' | 'Ý' => 'y',
            'ž' | 'Ž' => 'z',
            other => other.to_ascii_lowercase(),
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Orezaný text alebo `None`, ak je prázdny. Hodí sa pri čítaní stĺpcov.
pub fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_windows_1250() {
        // "Dátum" vo Windows-1250: á = 0xE1
        let bytes = b"D\xE1tum";
        assert_eq!(decode(bytes), "Dátum");
    }

    #[test]
    fn strips_utf8_bom() {
        assert_eq!(decode("\u{feff}Suma".as_bytes()), "Suma");
    }

    #[test]
    fn folds_diacritics_and_spaces() {
        assert_eq!(fold("  Dátum   ZAÚČTOVANIA "), "datum zauctovania");
        assert_eq!(fold("Ľubovoľný ŽĽTÝ kôň"), "lubovolny zlty kon");
    }
}
