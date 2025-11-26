//! Token representation for Tibetan text.
//!
//! A Token represents a segmented unit of text, which can be a word, punctuation,
//! or other text unit.

use serde::{Deserialize, Serialize};

/// The type of chunk/token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChunkType {
    /// Tibetan text (syllables/words)
    #[default]
    Text,
    /// Punctuation
    Punct,
    /// Number
    Num,
    /// Symbol
    Sym,
    /// Latin text
    Latin,
    /// CJK text
    Cjk,
    /// Other/unknown
    Other,
}

impl ChunkType {
    /// Convert to a string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkType::Text => "TEXT",
            ChunkType::Punct => "PUNCT",
            ChunkType::Num => "NUM",
            ChunkType::Sym => "SYM",
            ChunkType::Latin => "LATIN",
            ChunkType::Cjk => "CJK",
            ChunkType::Other => "OTHER",
        }
    }
}

/// A single token from the tokenization process
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Token {
    /// The raw text of the token
    pub text: String,

    /// Starting byte offset in the original string
    pub start: usize,

    /// Length in bytes
    pub len: usize,

    /// The type of this token
    pub chunk_type: ChunkType,

    /// Part-of-speech tag (if available)
    pub pos: Option<String>,

    /// Lemma (base form) of the word
    pub lemma: Option<String>,

    /// Frequency from dictionary
    pub freq: Option<u32>,

    /// Syllables that make up this token
    pub syls: Vec<String>,

    /// Whether this token is an affix
    pub is_affix: bool,

    /// Whether this token hosts an affix
    pub is_affix_host: bool,

    /// Whether this is a Sanskrit word
    pub is_skrt: bool,

    /// Additional senses/meanings from the dictionary
    pub senses: Vec<Sense>,

    /// Affixation information (if this token contains an affix)
    pub affixation: Option<AffixationInfo>,

    /// Whether this token has had a dagdra merged into it
    pub has_merged_dagdra: bool,
}

/// Information about affixation in a token
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AffixationInfo {
    /// Length of the affix in characters
    pub len: usize,
    /// Whether འ was removed before adding the affix
    pub aa: bool,
}

/// A word sense/meaning from the dictionary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sense {
    /// Part-of-speech for this sense
    pub pos: Option<String>,
    /// Lemma for this sense
    pub lemma: Option<String>,
    /// Frequency for this sense
    pub freq: Option<u32>,
    /// Sense description/gloss
    pub sense: Option<String>,
    /// Meaning/gloss (alias for sense)
    pub meaning: Option<String>,
    /// Whether this sense is for an affixed form
    pub affixed: bool,
}

impl Token {
    /// Create a new empty token
    pub fn new() -> Self {
        Token::default()
    }

    /// Create a token with text and position
    pub fn with_text(text: String, start: usize, len: usize, chunk_type: ChunkType) -> Self {
        Token {
            text,
            start,
            len,
            chunk_type,
            ..Default::default()
        }
    }

    /// Get the cleaned text (with proper tsek placement)
    pub fn text_cleaned(&self) -> String {
        if self.syls.is_empty() {
            return String::new();
        }

        let mut cleaned = self.syls.join("་");

        // Add trailing tsek unless it's an affix host without affix
        if self.is_affix_host && !self.is_affix {
            // Don't add tsek
        } else {
            cleaned.push('་');
        }

        cleaned
    }

    /// Check if this is a word token (TEXT type with syllables)
    pub fn is_word(&self) -> bool {
        self.chunk_type == ChunkType::Text && !self.syls.is_empty()
    }

    /// Check if this is punctuation
    pub fn is_punct(&self) -> bool {
        self.chunk_type == ChunkType::Punct
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)?;
        if let Some(ref pos) = self.pos {
            write!(f, "/{}", pos)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let token = Token::with_text("བཀྲ་ཤིས་".to_string(), 0, 18, ChunkType::Text);
        assert_eq!(token.text, "བཀྲ་ཤིས་");
        assert_eq!(token.start, 0);
        assert_eq!(token.chunk_type, ChunkType::Text);
    }

    #[test]
    fn test_token_display() {
        let mut token = Token::with_text("བཀྲ་ཤིས་".to_string(), 0, 18, ChunkType::Text);
        token.pos = Some("NOUN".to_string());
        assert_eq!(format!("{}", token), "བཀྲ་ཤིས་/NOUN");
    }
}

