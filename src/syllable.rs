//! Tibetan syllable analysis and affix system.
//!
//! This module provides functionality to analyze Tibetan syllables and generate
//! all possible affixed forms of a word.

use once_cell::sync::Lazy;
use regex::Regex;
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

/// Endings that indicate a syllable is NOT affixable
static NON_AFFIXABLE_ENDINGS: &[&str] = &["ར", "ས", "འི", "འོ", "མ", "ང"];

/// Dagdra particles (pa/po/ba/bo)
pub static DAGDRA: &[&str] = &["པ་", "པོ་", "བ་", "བོ་"];

/// Tsek character
pub const TSEK: char = '་';

/// Syllable components for determining if a syllable is affixable
pub struct SylComponents {
    /// Roots that can take affixes (simplified set)
    roots: HashSet<String>,
    /// Suffixes (used for syllable analysis)
    #[allow(dead_code)]
    suffixes: HashSet<String>,
    /// Regex for thame detection
    thame_regex: Regex,
}

impl Default for SylComponents {
    fn default() -> Self {
        Self::new()
    }
}

impl SylComponents {
    /// Create a new SylComponents with default data
    pub fn new() -> Self {
        // Load a simplified set of roots - in production, load from SylComponents.json
        let roots = Self::load_default_roots();
        let suffixes = Self::load_default_suffixes();
        
        // Regex pattern for thame (affixable syllables)
        // Matches syllables that can host affixed particles
        let thame_regex = Regex::new(
            r"([ྱྲླྭྷ]?[ིེོུ]?(འ?[ིོུ]?ར?ས?|(འ[མང])|(འོའ[མང])|(འིའ[ོམང])))$"
        ).expect("Invalid regex");

        SylComponents {
            roots,
            suffixes,
            thame_regex,
        }
    }

    fn load_default_roots() -> HashSet<String> {
        // Common Tibetan roots - this is a simplified set
        // In production, load from SylComponents.json
        let roots_str = include_str!("data/roots.txt");
        roots_str.lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|s| s.trim().to_string())
            .collect()
    }

    fn load_default_suffixes() -> HashSet<String> {
        let suffixes = [
            "འ", "ག", "གས", "ང", "ངས", "ད", "ན", "བ", "བས", "མ", "མས", "ལ",
            "འི", "འོ", "འང", "འམ", "ར", "ས", "འིའོ", "འིའམ", "འིའང", "འོའམ", "འོའང",
            "ི", "ིག", "ིགས", "ིང", "ིངས", "ིད", "ིན", "ིབ", "ིབས", "ིམ", "ིམས", "ིལ",
            "ིའི", "ིའོ", "ིའང", "ིའམ", "ིར", "ིས",
            "ུ", "ུག", "ུགས", "ུང", "ུངས", "ུད", "ུན", "ུབ", "ུབས", "ུམ", "ུམས", "ུལ",
            "ུའི", "ུའོ", "ུའང", "ུའམ", "ུར", "ུས",
            "ེ", "ེག", "ེགས", "ེང", "ེངས", "ེད", "ེན", "ེབ", "ེབས", "ེམ", "ེམས", "ེལ",
            "ེའི", "ེའོ", "ེའང", "ེའམ", "ེར", "ེས",
            "ོ", "ོག", "ོགས", "ོང", "ོངས", "ོད", "ོན", "ོབ", "ོབས", "ོམ", "ོམས", "ོལ",
            "ོའི", "ོའོ", "ོའང", "ོའམ", "ོར", "ོས",
        ];
        suffixes.iter().map(|s| s.to_string()).collect()
    }

    /// Check if a syllable is affixable (can take particle affixes)
    pub fn is_affixable(&self, syl: &str) -> bool {
        if !self.is_thame(syl) {
            return false;
        }

        // Check if it ends with a non-affixable ending
        for ending in NON_AFFIXABLE_ENDINGS {
            if syl.len() > ending.len() && syl.ends_with(ending) {
                return false;
            }
        }

        true
    }

    /// Check if a syllable is "thame" (can potentially host affixed particles)
    pub fn is_thame(&self, syl: &str) -> bool {
        // Simplified check - in production, use full get_info logic
        self.thame_regex.is_match(syl) || self.roots.contains(syl)
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
}

