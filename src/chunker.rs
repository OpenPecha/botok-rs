//! Chunking and syllabification for Tibetan text.
//!
//! This module segments text into chunks (syllables, punctuation, etc.) that can
//! then be processed by the tokenizer.

use crate::char_categories::{BoString, CharCategory};
use crate::token::ChunkType;

/// A chunk of text with its type and position
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The syllable text (cleaned, without tsek) - None for non-syllable chunks
    pub syl: Option<String>,
    /// The type of this chunk
    pub chunk_type: ChunkType,
    /// Starting byte offset in the original string
    pub start: usize,
    /// Length in bytes
    pub len: usize,
}

impl Chunk {
    /// Create a new chunk
    pub fn new(syl: Option<String>, chunk_type: ChunkType, start: usize, len: usize) -> Self {
        Chunk {
            syl,
            chunk_type,
            start,
            len,
        }
    }
}

/// Chunker for Tibetan text
pub struct Chunker {
    /// The analyzed string
    bs: BoString,
}

impl Chunker {
    /// Create a new chunker for the given string
    pub fn new(text: &str) -> Self {
        Chunker {
            bs: BoString::new(text),
        }
    }

    /// Get the original string
    pub fn string(&self) -> &str {
        &self.bs.string
    }

    /// Chunk the text into syllables, punctuation, etc.
    pub fn make_chunks(&self) -> Vec<Chunk> {
        if self.bs.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let chars: Vec<char> = self.bs.string.chars().collect();
        let mut byte_positions: Vec<usize> = Vec::with_capacity(chars.len() + 1);
        
        // Calculate byte positions for each character
        let mut pos = 0;
        for c in &chars {
            byte_positions.push(pos);
            pos += c.len_utf8();
        }
        byte_positions.push(pos); // End position

        let mut i = 0;
        while i < chars.len() {
            let cat = self.bs.categories[i];

            match cat {
                // Tibetan text - find the syllable
                CharCategory::Cons
                | CharCategory::SubCons
                | CharCategory::Vow
                | CharCategory::SkrtCons
                | CharCategory::SkrtSubCons
                | CharCategory::SkrtVow
                | CharCategory::SkrtLongVow
                | CharCategory::InSylMark
                | CharCategory::Nfc
                | CharCategory::NonBoNonSkrt => {
                    let (chunk, next_i) = self.read_syllable(&chars, &byte_positions, i);
                    chunks.push(chunk);
                    i = next_i;
                }

                // Tsek - usually attached to previous syllable, but handle standalone
                CharCategory::Tsek => {
                    // Standalone tsek (shouldn't happen often)
                    let start = byte_positions[i];
                    let len = chars[i].len_utf8();
                    chunks.push(Chunk::new(None, ChunkType::Punct, start, len));
                    i += 1;
                }

                // Punctuation
                CharCategory::NormalPunct | CharCategory::SpecialPunct => {
                    let (chunk, next_i) = self.read_punct(&chars, &byte_positions, i);
                    chunks.push(chunk);
                    i = next_i;
                }

                // Numbers
                CharCategory::Numeral => {
                    let (chunk, next_i) = self.read_numbers(&chars, &byte_positions, i);
                    chunks.push(chunk);
                    i = next_i;
                }

                // Symbols
                CharCategory::Symbol => {
                    let (chunk, next_i) = self.read_symbols(&chars, &byte_positions, i);
                    chunks.push(chunk);
                    i = next_i;
                }

                // Transparent (spaces) - attach to previous chunk or create standalone
                CharCategory::Transparent => {
                    // For now, skip spaces or attach to previous
                    if let Some(last) = chunks.last_mut() {
                        // Extend the previous chunk to include the space
                        last.len += chars[i].len_utf8();
                    }
                    i += 1;
                }

                // Latin text
                CharCategory::Latin => {
                    let (chunk, next_i) = self.read_latin(&chars, &byte_positions, i);
                    chunks.push(chunk);
                    i = next_i;
                }

                // CJK text
                CharCategory::Cjk => {
                    let (chunk, next_i) = self.read_cjk(&chars, &byte_positions, i);
                    chunks.push(chunk);
                    i = next_i;
                }

                // Other
                CharCategory::Other => {
                    let start = byte_positions[i];
                    let len = chars[i].len_utf8();
                    chunks.push(Chunk::new(None, ChunkType::Other, start, len));
                    i += 1;
                }
            }
        }

        chunks
    }

    /// Read a Tibetan syllable starting at position i
    fn read_syllable(
        &self,
        chars: &[char],
        byte_positions: &[usize],
        start_i: usize,
    ) -> (Chunk, usize) {
        let mut i = start_i;
        let mut syl_chars: Vec<char> = Vec::new();

        // Read until we hit a tsek or non-syllable character
        while i < chars.len() {
            let cat = self.bs.categories[i];

            match cat {
                // Part of syllable
                CharCategory::Cons
                | CharCategory::SubCons
                | CharCategory::Vow
                | CharCategory::SkrtCons
                | CharCategory::SkrtSubCons
                | CharCategory::SkrtVow
                | CharCategory::SkrtLongVow
                | CharCategory::InSylMark
                | CharCategory::Nfc
                | CharCategory::NonBoNonSkrt => {
                    syl_chars.push(chars[i]);
                    i += 1;
                }

                // Tsek ends the syllable (include it in the chunk but not the syl)
                CharCategory::Tsek => {
                    i += 1; // Include tsek in chunk length
                    break;
                }

                // Transparent (space) within syllable - include and continue
                CharCategory::Transparent => {
                    i += 1;
                    // Check if there's more syllable content after the space
                    if i < chars.len() && self.bs.categories[i].is_syllable_part() {
                        continue;
                    } else {
                        break;
                    }
                }

                // Anything else ends the syllable
                _ => break,
            }
        }

        let start = byte_positions[start_i];
        let end = byte_positions[i];
        let len = end - start;

        let syl = if syl_chars.is_empty() {
            None
        } else {
            Some(syl_chars.into_iter().collect())
        };

        (Chunk::new(syl, ChunkType::Text, start, len), i)
    }

    /// Read punctuation starting at position i
    fn read_punct(
        &self,
        chars: &[char],
        byte_positions: &[usize],
        start_i: usize,
    ) -> (Chunk, usize) {
        let mut i = start_i;

        while i < chars.len() {
            let cat = self.bs.categories[i];
            match cat {
                CharCategory::NormalPunct
                | CharCategory::SpecialPunct
                | CharCategory::Transparent => {
                    i += 1;
                }
                _ => break,
            }
        }

        let start = byte_positions[start_i];
        let end = byte_positions[i];
        (Chunk::new(None, ChunkType::Punct, start, end - start), i)
    }

    /// Read numbers starting at position i
    fn read_numbers(
        &self,
        chars: &[char],
        byte_positions: &[usize],
        start_i: usize,
    ) -> (Chunk, usize) {
        let mut i = start_i;

        while i < chars.len() {
            let cat = self.bs.categories[i];
            match cat {
                CharCategory::Numeral | CharCategory::Transparent => {
                    i += 1;
                }
                _ => break,
            }
        }

        let start = byte_positions[start_i];
        let end = byte_positions[i];
        (Chunk::new(None, ChunkType::Num, start, end - start), i)
    }

    /// Read symbols starting at position i
    fn read_symbols(
        &self,
        chars: &[char],
        byte_positions: &[usize],
        start_i: usize,
    ) -> (Chunk, usize) {
        let mut i = start_i;

        while i < chars.len() {
            let cat = self.bs.categories[i];
            match cat {
                CharCategory::Symbol | CharCategory::Transparent => {
                    i += 1;
                }
                _ => break,
            }
        }

        let start = byte_positions[start_i];
        let end = byte_positions[i];
        (Chunk::new(None, ChunkType::Sym, start, end - start), i)
    }

    /// Read Latin text starting at position i
    fn read_latin(
        &self,
        chars: &[char],
        byte_positions: &[usize],
        start_i: usize,
    ) -> (Chunk, usize) {
        let mut i = start_i;

        while i < chars.len() {
            let cat = self.bs.categories[i];
            match cat {
                CharCategory::Latin | CharCategory::Transparent => {
                    i += 1;
                }
                _ => break,
            }
        }

        let start = byte_positions[start_i];
        let end = byte_positions[i];
        (Chunk::new(None, ChunkType::Latin, start, end - start), i)
    }

    /// Read CJK text starting at position i
    fn read_cjk(
        &self,
        chars: &[char],
        byte_positions: &[usize],
        start_i: usize,
    ) -> (Chunk, usize) {
        let mut i = start_i;

        while i < chars.len() {
            let cat = self.bs.categories[i];
            match cat {
                CharCategory::Cjk | CharCategory::Transparent => {
                    i += 1;
                }
                _ => break,
            }
        }

        let start = byte_positions[start_i];
        let end = byte_positions[i];
        (Chunk::new(None, ChunkType::Cjk, start, end - start), i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_syllables() {
        let chunker = Chunker::new("བཀྲ་ཤིས་");
        let chunks = chunker.make_chunks();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].syl, Some("བཀྲ".to_string()));
        assert_eq!(chunks[0].chunk_type, ChunkType::Text);
        assert_eq!(chunks[1].syl, Some("ཤིས".to_string()));
    }

    #[test]
    fn test_with_punctuation() {
        let chunker = Chunker::new("བཀྲ་ཤིས།");
        let chunks = chunker.make_chunks();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].syl, Some("བཀྲ".to_string()));
        assert_eq!(chunks[1].syl, Some("ཤིས".to_string()));
        assert_eq!(chunks[2].chunk_type, ChunkType::Punct);
    }

    #[test]
    fn test_mixed_content() {
        let chunker = Chunker::new("བཀྲ་ཤིས། hello");
        let chunks = chunker.make_chunks();

        // Should have: syllable, syllable, punct, latin
        assert!(chunks.len() >= 3);
        assert_eq!(chunks[0].chunk_type, ChunkType::Text);
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Latin));
    }

    #[test]
    fn test_chunk_positions() {
        let text = "བཀྲ་";
        let chunker = Chunker::new(text);
        let chunks = chunker.make_chunks();

        assert_eq!(chunks.len(), 1);
        let chunk = &chunks[0];
        assert_eq!(&text[chunk.start..chunk.start + chunk.len], "བཀྲ་");
    }
}

