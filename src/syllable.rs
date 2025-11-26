//! Tibetan syllable analysis and affix system.
//!
//! This module provides functionality to analyze Tibetan syllables and generate
//! all possible affixed forms of a word.

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// Information about an affix
#[derive(Debug, Clone)]
pub struct AffixData {
    /// Length of the affix in characters
    pub len: usize,
    /// Type of affix (e.g., "la", "gis", "gi", etc.)
    pub affix_type: String,
    /// Whether འ was removed before adding the affix
    pub aa: bool,
}

/// All possible Tibetan affixes
static AFFIXES: Lazy<HashMap<&'static str, (usize, &'static str)>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ར", (1, "la"));
    m.insert("ས", (1, "gis"));
    m.insert("འི", (2, "gi"));
    m.insert("འམ", (2, "am"));
    m.insert("འང", (2, "ang"));
    m.insert("འོ", (2, "o"));
    m.insert("འིའོ", (4, "gi+o"));
    m.insert("འིའམ", (4, "gi+am"));
    m.insert("འིའང", (4, "gi+ang"));
    m.insert("འོའམ", (4, "o+am"));
    m.insert("འོའང", (4, "o+ang"));
    m
});

/// Affixes that indicate a syllable is already affixed (NOT affixable)
/// These are the grammatical particle suffixes, not syllable-internal suffixes
static AFFIX_PARTICLES: &[&str] = &[
    "འི",   // genitive
    "འོ",   // terminative
    "འམ",   // alternative
    "འང",   // concessive
    "འིའོ", // genitive + terminative
    "འིའམ", // genitive + alternative
    "འིའང", // genitive + concessive
    "འོའམ", // terminative + alternative
    "འོའང", // terminative + concessive
];

/// Suffixes that indicate a syllable can potentially take affixes
/// These are the valid Tibetan syllable-final suffixes that can host particles
static AFFIXABLE_SUFFIXES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let suffixes = [
        // Basic suffixes that can take affixes
        "འ", "ག", "ང", "ད", "ན", "བ", "མ", "ལ",
        // With vowels
        "ིག", "ིང", "ིད", "ིན", "ིབ", "ིམ", "ིལ", "ིས",
        "ུག", "ུང", "ུད", "ུན", "ུབ", "ུམ", "ུལ", "ུས",
        "ེག", "ེང", "ེད", "ེན", "ེབ", "ེམ", "ེལ", "ེས",
        "ོག", "ོང", "ོད", "ོན", "ོབ", "ོམ", "ོལ", "ོས",
        // Just vowels (open syllables)
        "ི", "ུ", "ེ", "ོ",
        // Standalone consonant suffixes
        "ས", "ར",
    ];
    suffixes.iter().copied().collect()
});

/// Dagdra particles (pa/po/ba/bo)
pub static DAGDRA: &[&str] = &["པ་", "པོ་", "བ་", "བོ་"];

/// Tsek character
pub const TSEK: char = '་';

/// Syllable components for determining if a syllable is affixable
pub struct SylComponents {
    /// Roots that can take affixes
    roots: HashSet<String>,
}

impl Default for SylComponents {
    fn default() -> Self {
        Self::new()
    }
}

impl SylComponents {
    /// Create a new SylComponents with default data
    pub fn new() -> Self {
        let roots = Self::load_default_roots();
        SylComponents { roots }
    }

    fn load_default_roots() -> HashSet<String> {
        // Load roots from data file
        let roots_str = include_str!("data/roots.txt");
        roots_str.lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Check if a syllable is affixable (can take particle affixes)
    /// 
    /// A syllable is affixable if:
    /// 1. It's not already affixed (doesn't end with འི, འོ, འམ, འང, etc.)
    /// 2. It ends with a valid suffix that can host affixes
    pub fn is_affixable(&self, syl: &str) -> bool {
        // Check if it already ends with an affix particle (not affixable)
        for affix in AFFIX_PARTICLES {
            if syl.len() > affix.len() && syl.ends_with(affix) {
                return false;
            }
        }

        // Check if it ends with a valid affixable suffix
        // Or if it's a root syllable
        self.is_thame(syl)
    }

    /// Check if a syllable is "thame" (can potentially host affixed particles)
    /// 
    /// This uses a simplified heuristic:
    /// - Check if the syllable ends with a valid suffix
    /// - Check if the syllable is a known root
    pub fn is_thame(&self, syl: &str) -> bool {
        // Check if it's a known root
        if self.roots.contains(syl) {
            return true;
        }

        // Check if it ends with a valid affixable suffix
        // We check from longest to shortest
        for suffix_len in (1..=4).rev() {
            if syl.chars().count() > suffix_len {
                let suffix: String = syl.chars().rev().take(suffix_len).collect::<Vec<_>>().into_iter().rev().collect();
                if AFFIXABLE_SUFFIXES.contains(suffix.as_str()) {
                    return true;
                }
            }
        }

        // Check if it ends with འ (which gets removed before affixation)
        if syl.ends_with('འ') && syl.chars().count() > 1 {
            return true;
        }

        false
    }

    /// Get all affixed forms of a syllable
    /// 
    /// Returns None if the syllable is not affixable.
    /// Otherwise returns a Vec of (affixed_form, affix_data) tuples.
    pub fn get_all_affixed(&self, syl: &str) -> Option<Vec<(String, AffixData)>> {
        if !self.is_affixable(syl) {
            return None;
        }

        let mut aa = false;
        let base_syl = if syl.ends_with('འ') && syl.chars().count() > 1 {
            aa = true;
            // Remove the trailing འ
            let mut chars: Vec<char> = syl.chars().collect();
            chars.pop();
            chars.into_iter().collect()
        } else {
            syl.to_string()
        };

        let mut affixed = Vec::new();
        for (affix, (len, affix_type)) in AFFIXES.iter() {
            let affixed_form = format!("{}{}", base_syl, affix);
            affixed.push((
                affixed_form,
                AffixData {
                    len: *len,
                    affix_type: affix_type.to_string(),
                    aa,
                },
            ));
        }

        Some(affixed)
    }
}

/// Check if a word is a dagdra particle (pa/po/ba/bo)
pub fn is_dagdra(text: &str) -> bool {
    let cleaned = if text.ends_with(TSEK) {
        text.to_string()
    } else {
        format!("{}{}", text, TSEK)
    };
    DAGDRA.contains(&cleaned.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affixes() {
        assert_eq!(AFFIXES.get("ར"), Some(&(1, "la")));
        assert_eq!(AFFIXES.get("འི"), Some(&(2, "gi")));
    }

    #[test]
    fn test_is_dagdra() {
        assert!(is_dagdra("པ་"));
        assert!(is_dagdra("པོ་"));
        assert!(is_dagdra("བ་"));
        assert!(is_dagdra("བོ་"));
        assert!(is_dagdra("པ")); // Without tsek
        assert!(!is_dagdra("ཀ་"));
    }

    #[test]
    fn test_is_affixable() {
        let sc = SylComponents::new();
        
        // Test syllables that should be affixable
        assert!(sc.is_affixable("ཤིས"), "ཤིས should be affixable (ends with ིས which is a valid suffix)");
        assert!(sc.is_affixable("བཀྲ"), "བཀྲ should be affixable (it's a root)");
        assert!(sc.is_affixable("ལེགས"), "ལེགས should be affixable");
        
        // Test syllables that should NOT be affixable (already affixed with particle)
        // Note: These are single syllables with affixes attached, not multi-syllable words
        assert!(!sc.is_affixable("ཤིསའི"), "ཤིསའི should NOT be affixable (ends with འི genitive)");
        assert!(!sc.is_affixable("བཀྲའོ"), "བཀྲའོ should NOT be affixable (ends with འོ terminative)");
    }

    #[test]
    fn test_get_all_affixed() {
        let sc = SylComponents::new();
        
        // Test that affixed forms are generated
        let affixed = sc.get_all_affixed("ཤིས");
        assert!(affixed.is_some(), "ཤིས should generate affixed forms");
        
        let forms = affixed.unwrap();
        assert!(!forms.is_empty(), "Should have generated at least one affixed form");
        
        // Check that specific affixes are generated (without tsek - affixes attach directly)
        let affix_forms: Vec<&str> = forms.iter().map(|(f, _)| f.as_str()).collect();
        println!("Generated affixed forms: {:?}", affix_forms);
        assert!(affix_forms.contains(&"ཤིསར"), "Should contain ཤིསར (la affix)");
        assert!(affix_forms.contains(&"ཤིསས"), "Should contain ཤིསས (gis affix)");
        assert!(affix_forms.contains(&"ཤིསའི"), "Should contain ཤིསའི (gi affix)");
    }
}

