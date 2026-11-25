//! Automatická kategorizácia podľa pravidiel "kľúčové slovo → kategória".
//!
//! Pravidlo sa uplatní, ak sa jeho vzor nachádza v protistrane alebo
//! v popise transakcie (bez ohľadu na veľkosť písmen a diakritiku).
//! Ak sedí viac pravidiel, vyhráva **najdlhší vzor** – takže "BOLT FOOD"
//! (Reštaurácie) má prednosť pred "BOLT" (Doprava).

use crate::parser::text::fold;

/// Pravidlo v podobe, akú potrebuje kategorizácia (bez id a názvu kategórie).
#[derive(Debug, Clone)]
pub struct MatchRule {
    /// Vzor už prevedený cez `fold`, aby sme to nerobili pri každej transakcii.
    pub folded_pattern: String,
    pub category_id: i64,
}

impl MatchRule {
    pub fn new(pattern: &str, category_id: i64) -> Self {
        MatchRule {
            folded_pattern: fold(pattern),
            category_id,
        }
    }
}

/// Text, v ktorom hľadáme vzory: protistrana + popis.
pub fn searchable_text(counterparty: Option<&str>, description: Option<&str>) -> String {
    let joined = [counterparty, description]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    fold(&joined)
}

/// Vráti id kategórie podľa najdlhšieho zhodného vzoru, alebo `None`.
pub fn categorize(text: &str, rules: &[MatchRule]) -> Option<i64> {
    rules
        .iter()
        .filter(|r| !r.folded_pattern.is_empty() && text.contains(&r.folded_pattern))
        .max_by_key(|r| r.folded_pattern.len())
        .map(|r| r.category_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOPRAVA: i64 = 1;
    const RESTAURACIE: i64 = 2;
    const POTRAVINY: i64 = 3;

    fn rules() -> Vec<MatchRule> {
        vec![
            MatchRule::new("BOLT", DOPRAVA),
            MatchRule::new("Bolt Food", RESTAURACIE),
            MatchRule::new("lidl", POTRAVINY),
        ]
    }

    #[test]
    fn longest_pattern_wins() {
        let text = searchable_text(Some("BOLT FOOD"), Some("Platba kartou"));
        assert_eq!(categorize(&text, &rules()), Some(RESTAURACIE));

        let text = searchable_text(Some("BOLT.EU/O/2611"), None);
        assert_eq!(categorize(&text, &rules()), Some(DOPRAVA));
    }

    #[test]
    fn matches_in_description_ignoring_case() {
        let text = searchable_text(None, Some("Platba kartou Lidl Bratislava"));
        assert_eq!(categorize(&text, &rules()), Some(POTRAVINY));
    }

    #[test]
    fn no_match_returns_none() {
        let text = searchable_text(Some("Kaviareň"), None);
        assert_eq!(categorize(&text, &rules()), None);
    }
}
