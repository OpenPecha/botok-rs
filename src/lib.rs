//! # botok-rs
//!
//! A fast Tibetan tokenizer written in Rust.
//!
//! This is a Rust port of the Python [botok](https://github.com/OpenPecha/botok) library.
//! It provides efficient tokenization of Tibetan text using a dictionary-based
//! longest-match algorithm.
//!
//! ## Quick Start
//!
//! ```rust
//! use botok_rs::{Tokenizer, TrieBuilder};
//!
//! // Build a trie from TSV data
//! let tsv = "བཀྲ་ཤིས\tNOUN\t\t\t1000\nབདེ་ལེགས\tNOUN\t\t\t500";
//! let mut builder = TrieBuilder::new();
//! builder.load_tsv(tsv);
//! let trie = builder.build();
//!
//! // Create tokenizer and tokenize text
//! let tokenizer = Tokenizer::new(trie);
//! let tokens = tokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།");
//!
//! for token in &tokens {
//!     println!("{}: {:?}", token.text, token.pos);
//! }
//! ```
//!
//! ## Simple Tokenization (No Dictionary)
//!
//! If you just need syllable-level tokenization without a dictionary:
//!
//! ```rust
//! use botok_rs::SimpleTokenizer;
//!
//! let tokens = SimpleTokenizer::tokenize("བཀྲ་ཤིས་བདེ་ལེགས།");
//! for token in &tokens {
//!     println!("{}", token.text);
//! }
//! ```
//!
//! ## Python Bindings
//!
//! This library can be compiled as a Python extension module. See the README for details.

pub mod char_categories;
pub mod chunker;
pub mod token;
pub mod tokenizer;
pub mod trie;

// Python bindings (only compiled when the "python" feature is enabled)
#[cfg(feature = "python")]
pub mod python;

// Re-export main types for convenience
pub use char_categories::{get_char_category, BoString, CharCategory};
pub use chunker::{Chunk, Chunker};
pub use token::{ChunkType, Sense, Token};
pub use tokenizer::{SimpleTokenizer, Tokenizer};
pub use trie::{AffixInfo, Trie, TrieBuilder, TrieNode, WordData};

/// Version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline() {
        // Build a simple trie
        let tsv = r#"བཀྲ་ཤིས	NOUN			1000
བདེ་ལེགས	NOUN			500
བཀྲ་ཤིས་བདེ་ལེགས	PHRASE			2000"#;

        let mut builder = TrieBuilder::new();
        builder.load_tsv(tsv);
        let trie = builder.build();

        // Tokenize
        let tokenizer = Tokenizer::new(trie);
        let tokens = tokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས། བཀྲ་ཤིས།");

        // Verify results
        assert!(!tokens.is_empty());

        // First token should be the full phrase (longest match)
        assert_eq!(tokens[0].syls.len(), 4);

        // Should have punctuation
        assert!(tokens.iter().any(|t| t.chunk_type == ChunkType::Punct));
    }

    #[test]
    fn test_simple_tokenizer() {
        let tokens = SimpleTokenizer::tokenize("བཀྲ་ཤིས་བདེ་ལེགས།");

        // Should have 4 syllables + 1 punctuation
        assert_eq!(tokens.len(), 5);
    }
}
