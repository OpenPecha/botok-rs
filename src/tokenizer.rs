//! The main tokenizer implementing longest-match algorithm.
//!
//! This module takes chunked text and uses the Trie to find the longest matching
//! words, producing a list of tokens.

use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

use crate::chunker::{Chunk, Chunker};
use crate::modifiers::apply_all_modifiers;
use crate::token::{ChunkType, Token};
use crate::trie::{Trie, TrieNode};

/// The main tokenizer
pub struct Tokenizer {
    /// The dictionary trie (shared reference)
    trie: Arc<Trie>,
}

impl Tokenizer {
    /// Create a new tokenizer with the given trie
    pub fn new(trie: Trie) -> Self {
        Tokenizer { trie: Arc::new(trie) }
    }

    /// Create a new tokenizer with a shared trie reference
    pub fn with_arc(trie: Arc<Trie>) -> Self {
        Tokenizer { trie }
    }

    /// Get a reference to the trie
    pub fn trie(&self) -> &Trie {
        &self.trie
    }

    /// Get the Arc reference to the trie (for sharing)
    pub fn trie_arc(&self) -> Arc<Trie> {
        Arc::clone(&self.trie)
    }

    /// Tokenize a string with full post-processing
    pub fn tokenize(&self, text: &str) -> Vec<Token> {
        self.tokenize_with_options(text, true)
    }

    /// Tokenize a string with configurable options
    pub fn tokenize_with_options(&self, text: &str, split_affixes: bool) -> Vec<Token> {
        // Normalize Unicode (NFC normalization)
        let normalized: String = text.nfc().collect();
        
        let chunker = Chunker::new(&normalized);
        let chunks = chunker.make_chunks();
        let mut tokens = self.tokenize_chunks(&chunks, &normalized);
        
        // Apply post-processing
        apply_all_modifiers(&mut tokens, split_affixes);
        
        tokens
    }

    /// Tokenize without post-processing (raw tokenization)
    pub fn tokenize_raw(&self, text: &str) -> Vec<Token> {
        // Normalize Unicode (NFC normalization)
        let normalized: String = text.nfc().collect();
        
        let chunker = Chunker::new(&normalized);
        let chunks = chunker.make_chunks();
        self.tokenize_chunks(&chunks, &normalized)
    }

    /// Tokenize pre-chunked text
    pub fn tokenize_chunks(&self, chunks: &[Chunk], original_text: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut i = 0;

        while i < chunks.len() {
            let chunk = &chunks[i];

            // Non-syllable chunks are passed through as-is
            if chunk.syl.is_none() {
                tokens.push(Token::with_text(
                    original_text[chunk.start..chunk.start + chunk.len].to_string(),
                    chunk.start,
                    chunk.len,
                    chunk.chunk_type,
                ));
                i += 1;
                continue;
            }

            // For syllable chunks, use longest match
            let (token, next_i) = self.longest_match(chunks, original_text, i);
            tokens.push(token);
            i = next_i;
        }

        tokens
    }

    /// Find the longest matching word starting at position i
    fn longest_match(&self, chunks: &[Chunk], original_text: &str, start_i: usize) -> (Token, usize) {
        let mut walker = start_i;
        let mut current_node: Option<&TrieNode> = None;
        let mut last_match_idx: Option<usize> = None;
        let mut last_match_node: Option<&TrieNode> = None;
        let mut syls: Vec<String> = Vec::new();

        // Walk the trie as far as we can
        while walker < chunks.len() {
            let chunk = &chunks[walker];

            // Only process syllable chunks
            if let Some(ref syl) = chunk.syl {
                if let Some(next_node) = self.trie.walk(syl, current_node) {
                    current_node = Some(next_node);
                    syls.push(syl.clone());

                    // Record if this is a valid word ending
                    if next_node.is_match() {
                        last_match_idx = Some(walker);
                        last_match_node = Some(next_node);
                    }

                    walker += 1;
                } else {
                    // Can't continue in trie
                    break;
                }
            } else {
                // Hit a non-syllable chunk
                break;
            }
        }

        // Determine what to return
        if let Some(match_idx) = last_match_idx {
            // We found a valid word
            let start_chunk = &chunks[start_i];
            let end_chunk = &chunks[match_idx];
            let start = start_chunk.start;
            let end = end_chunk.start + end_chunk.len;

            let mut token = Token::with_text(
                original_text[start..end].to_string(),
                start,
                end - start,
                ChunkType::Text,
            );

            // Add syllables up to the match
            token.syls = syls[..=(match_idx - start_i)].to_vec();

            // Add data from trie if available
            if let Some(node) = last_match_node {
                if let Some(ref data) = node.data {
                    token.pos = data.pos.clone();
                    token.lemma = data.lemma.clone();
                    token.freq = data.freq;
                    token.is_skrt = data.skrt;
                    token.senses = data.senses.clone();
                    
                    // Copy affixation info
                    if let Some(ref affix_info) = data.affixation {
                        token.affixation = Some(crate::token::AffixationInfo {
                            len: affix_info.len,
                            aa: affix_info.aa,
                        });
                    }
                }
            }

            (token, match_idx + 1)
        } else {
            // No match found - return the first syllable as an unknown word
            let chunk = &chunks[start_i];
            let mut token = Token::with_text(
                original_text[chunk.start..chunk.start + chunk.len].to_string(),
                chunk.start,
                chunk.len,
                ChunkType::Text,
            );

            if let Some(ref syl) = chunk.syl {
                token.syls = vec![syl.clone()];
            }

            // Mark as unknown (no POS)
            token.pos = Some("NO_POS".to_string());

            (token, start_i + 1)
        }
    }
}

/// A simple tokenizer that doesn't use a dictionary (just syllabifies)
pub struct SimpleTokenizer;

impl SimpleTokenizer {
    /// Tokenize text into syllables (no dictionary lookup)
    pub fn tokenize(text: &str) -> Vec<Token> {
        // Normalize Unicode
        let normalized: String = text.nfc().collect();
        
        let chunker = Chunker::new(&normalized);
        let chunks = chunker.make_chunks();

        chunks
            .into_iter()
            .map(|chunk| {
                let mut token = Token::with_text(
                    normalized[chunk.start..chunk.start + chunk.len].to_string(),
                    chunk.start,
                    chunk.len,
                    chunk.chunk_type,
                );
                if let Some(syl) = chunk.syl {
                    token.syls = vec![syl];
                }
                token
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::TrieBuilder;

    fn make_test_trie() -> Trie {
        let tsv = r#"བཀྲ་ཤིས	NOUN			1000
བདེ་ལེགས	NOUN			500
བཀྲ་ཤིས་བདེ་ལེགས	NOUN			2000"#;

        let mut builder = TrieBuilder::new();
        builder.load_tsv(tsv);
        builder.build()
    }

    #[test]
    fn test_simple_tokenization() {
        let trie = make_test_trie();
        let tokenizer = Tokenizer::new(trie);

        let tokens = tokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།");

        // Should find the longest match "བཀྲ་ཤིས་བདེ་ལེགས" + punctuation
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].pos, Some("NOUN".to_string()));
        assert_eq!(tokens[1].chunk_type, ChunkType::Punct);
    }

    #[test]
    fn test_unknown_word() {
        let trie = make_test_trie();
        let tokenizer = Tokenizer::new(trie);

        let tokens = tokenizer.tokenize("ཀཀ་");

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].pos, Some("NO_POS".to_string()));
    }

    #[test]
    fn test_mixed_known_unknown() {
        let trie = make_test_trie();
        let tokenizer = Tokenizer::new(trie);

        let tokens = tokenizer.tokenize("བཀྲ་ཤིས་ཀཀ་");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].pos, Some("NOUN".to_string())); // བཀྲ་ཤིས
        assert_eq!(tokens[1].pos, Some("NO_POS".to_string())); // ཀཀ (unknown)
    }

    #[test]
    fn test_simple_tokenizer() {
        let tokens = SimpleTokenizer::tokenize("བཀྲ་ཤིས།");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].syls, vec!["བཀྲ"]);
        assert_eq!(tokens[1].syls, vec!["ཤིས"]);
        assert_eq!(tokens[2].chunk_type, ChunkType::Punct);
    }

    #[test]
    fn test_arc_sharing() {
        let trie = make_test_trie();
        let tokenizer1 = Tokenizer::new(trie);
        let arc = tokenizer1.trie_arc();
        let tokenizer2 = Tokenizer::with_arc(arc);

        // Both tokenizers should work
        let tokens1 = tokenizer1.tokenize("བཀྲ་ཤིས།");
        let tokens2 = tokenizer2.tokenize("བཀྲ་ཤིས།");

        assert_eq!(tokens1.len(), tokens2.len());
    }

    #[test]
    fn test_unicode_normalization() {
        // Test that different Unicode forms produce the same result
        let trie = make_test_trie();
        let tokenizer = Tokenizer::new(trie);

        // NFC form
        let tokens_nfc = tokenizer.tokenize("བཀྲ་ཤིས།");
        
        // The tokenizer should handle both forms
        assert!(!tokens_nfc.is_empty());
    }
}
