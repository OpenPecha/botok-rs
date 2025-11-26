//! Character classification for Tibetan Unicode characters.
//!
//! This module provides functionality to categorize each character in a Tibetan string
//! into its appropriate category (consonant, vowel, punctuation, etc.).

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Character categories used in Tibetan text processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CharCategory {
    /// Tibetan consonant
    Cons,
    /// Sub-joined consonant (used in consonant clusters)
    SubCons,
    /// Tibetan vowel sign
    Vow,
    /// Tsek (syllable separator ་)
    Tsek,
    /// Normal punctuation (shad, etc.)
    NormalPunct,
    /// Special punctuation
    SpecialPunct,
    /// Tibetan numeral
    Numeral,
    /// Symbol
    Symbol,
    /// Transparent characters (spaces, etc.) - ignored in syllable processing
    Transparent,
    /// Sanskrit vowel
    SkrtVow,
    /// Sanskrit consonant
    SkrtCons,
    /// Sanskrit sub-joined consonant
    SkrtSubCons,
    /// Sanskrit long vowel (visarga ཿ)
    SkrtLongVow,
    /// In-syllable mark
    InSylMark,
    /// NFC (pre-composed) character
    Nfc,
    /// Non-Tibetan, non-Sanskrit character
    NonBoNonSkrt,
    /// Latin character
    Latin,
    /// CJK character
    Cjk,
    /// Other/unknown character
    #[default]
    Other,
}

impl CharCategory {
    /// Parse a category string from the CSV file
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "CONS" => CharCategory::Cons,
            "SUB_CONS" => CharCategory::SubCons,
            "VOW" => CharCategory::Vow,
            "TSEK" => CharCategory::Tsek,
            "NORMAL_PUNCT" => CharCategory::NormalPunct,
            "SPECIAL_PUNCT" => CharCategory::SpecialPunct,
            "NUMERAL" => CharCategory::Numeral,
            "SYMBOL" => CharCategory::Symbol,
            "SKRT_VOW" => CharCategory::SkrtVow,
            "SKRT_CONS" => CharCategory::SkrtCons,
            "SKRT_SUB_CONS" => CharCategory::SkrtSubCons,
            "SKRT_LONG_VOW" => CharCategory::SkrtLongVow,
            "IN_SYL_MARK" => CharCategory::InSylMark,
            "NFC" => CharCategory::Nfc,
            "NON_BO_NON_SKRT" => CharCategory::NonBoNonSkrt,
            _ => CharCategory::Other,
        }
    }

    /// Check if this category represents a character that can be part of a syllable
    pub fn is_syllable_part(&self) -> bool {
        matches!(
            self,
            CharCategory::Cons
                | CharCategory::SubCons
                | CharCategory::Vow
                | CharCategory::SkrtVow
                | CharCategory::SkrtCons
                | CharCategory::SkrtSubCons
                | CharCategory::SkrtLongVow
                | CharCategory::InSylMark
                | CharCategory::Nfc
                | CharCategory::NonBoNonSkrt
        )
    }

    /// Check if this is a Tibetan character (not Latin, CJK, or Other)
    pub fn is_tibetan(&self) -> bool {
        !matches!(
            self,
            CharCategory::Latin | CharCategory::Cjk | CharCategory::Other
        )
    }
}

/// Embedded character table from bo_uni_table.csv
static BO_UNI_TABLE: &str = include_str!("data/bo_uni_table.csv");

/// Lazily initialized map from character to category
static CHAR_MAP: Lazy<HashMap<char, CharCategory>> = Lazy::new(|| {
    let mut map = HashMap::new();

    for line in BO_UNI_TABLE.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 3 {
            // Parse the Unicode code point (first column, e.g., "0F40")
            if let Ok(code_point) = u32::from_str_radix(parts[0].trim(), 16) {
                if let Some(c) = char::from_u32(code_point) {
                    let category = CharCategory::from_str(parts[2]);
                    map.insert(c, category);
                }
            }
        }
    }

    map
});

/// List of characters that should be treated as transparent (spaces, etc.)
const TRANSPARENT_CHARS: &[char] = &[
    ' ',      // SPACE
    '\t',     // TAB
    '\n',     // NEWLINE
    '\r',     // CARRIAGE RETURN
    '\u{00A0}', // NO-BREAK SPACE
    '\u{1680}', // OGHAM SPACE MARK
    '\u{2000}', // EN QUAD
    '\u{2001}', // EM QUAD
    '\u{2002}', // EN SPACE
    '\u{2003}', // EM SPACE
    '\u{2004}', // THREE-PER-EM SPACE
    '\u{2005}', // FOUR-PER-EM SPACE
    '\u{2006}', // SIX-PER-EM SPACE
    '\u{2007}', // FIGURE SPACE
    '\u{2008}', // PUNCTUATION SPACE
    '\u{2009}', // THIN SPACE
    '\u{200A}', // HAIR SPACE
    '\u{200B}', // ZERO WIDTH SPACE
    '\u{202F}', // NARROW NO-BREAK SPACE
    '\u{205F}', // MEDIUM MATHEMATICAL SPACE
    '\u{3000}', // IDEOGRAPHIC SPACE
    '\u{FEFF}', // ZERO WIDTH NO-BREAK SPACE
];

/// Get the category of a character
pub fn get_char_category(c: char) -> CharCategory {
    // Check for transparent (space-like) characters first
    if TRANSPARENT_CHARS.contains(&c) {
        return CharCategory::Transparent;
    }

    // Check the Tibetan Unicode range (U+0F00 to U+0FFF)
    if ('\u{0F00}'..='\u{0FFF}').contains(&c) {
        return *CHAR_MAP.get(&c).unwrap_or(&CharCategory::Other);
    }

    // Check Latin range
    // Basic Latin + Latin-1 Supplement + Latin Extended-A/B + IPA Extensions + Spacing Modifier Letters
    if ('\u{0020}'..='\u{036F}').contains(&c) || ('\u{1E00}'..='\u{20CF}').contains(&c) {
        return CharCategory::Latin;
    }

    // Check CJK range (simplified check for common CJK blocks)
    if ('\u{4E00}'..='\u{9FFF}').contains(&c)     // CJK Unified Ideographs
        || ('\u{3400}'..='\u{4DBF}').contains(&c) // CJK Unified Ideographs Extension A
        || ('\u{2E80}'..='\u{2EFF}').contains(&c) // CJK Radicals Supplement
        || ('\u{3000}'..='\u{303F}').contains(&c) // CJK Symbols and Punctuation
    {
        return CharCategory::Cjk;
    }

    CharCategory::Other
}

/// A string with character category information for each character
#[derive(Debug, Clone)]
pub struct BoString {
    /// The original string
    pub string: String,
    /// Category for each character (by index)
    pub categories: Vec<CharCategory>,
}

impl BoString {
    /// Create a new BoString from a string
    pub fn new(s: &str) -> Self {
        let categories: Vec<CharCategory> = s.chars().map(get_char_category).collect();
        BoString {
            string: s.to_string(),
            categories,
        }
    }

    /// Get the length (number of characters)
    pub fn len(&self) -> usize {
        self.categories.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.categories.is_empty()
    }

    /// Get the category at a specific index
    pub fn get_category(&self, idx: usize) -> Option<CharCategory> {
        self.categories.get(idx).copied()
    }

    /// Get a slice of categories
    pub fn get_categories(&self, start: usize, len: usize) -> &[CharCategory] {
        let end = (start + len).min(self.categories.len());
        &self.categories[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tibetan_consonants() {
        assert_eq!(get_char_category('ཀ'), CharCategory::Cons);
        assert_eq!(get_char_category('ག'), CharCategory::Cons);
        assert_eq!(get_char_category('བ'), CharCategory::Cons);
    }

    #[test]
    fn test_tibetan_vowels() {
        assert_eq!(get_char_category('ི'), CharCategory::Vow);
        assert_eq!(get_char_category('ུ'), CharCategory::Vow);
        assert_eq!(get_char_category('ེ'), CharCategory::Vow);
        assert_eq!(get_char_category('ོ'), CharCategory::Vow);
    }

    #[test]
    fn test_tsek() {
        assert_eq!(get_char_category('་'), CharCategory::Tsek);
    }

    #[test]
    fn test_punctuation() {
        assert_eq!(get_char_category('།'), CharCategory::NormalPunct);
    }

    #[test]
    fn test_space() {
        assert_eq!(get_char_category(' '), CharCategory::Transparent);
        assert_eq!(get_char_category('\t'), CharCategory::Transparent);
    }

    #[test]
    fn test_latin() {
        assert_eq!(get_char_category('a'), CharCategory::Latin);
        assert_eq!(get_char_category('Z'), CharCategory::Latin);
    }

    #[test]
    fn test_bo_string() {
        let bs = BoString::new("བཀྲ་");
        assert_eq!(bs.len(), 4);
        assert_eq!(bs.get_category(0), Some(CharCategory::Cons)); // བ
        assert_eq!(bs.get_category(1), Some(CharCategory::Cons)); // ཀ
        assert_eq!(bs.get_category(2), Some(CharCategory::SubCons)); // ྲ
        assert_eq!(bs.get_category(3), Some(CharCategory::Tsek)); // ་
    }
}

