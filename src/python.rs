//! Python bindings for botok-rs using PyO3
//!
//! This module provides Python-compatible wrappers around the Rust tokenizer.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::chunker::Chunker;
use crate::token::{ChunkType, Token as RustToken};
use crate::tokenizer::{SimpleTokenizer as RustSimpleTokenizer, Tokenizer as RustTokenizer};
use crate::trie::{Trie, TrieBuilder, WordData};

/// A Python-compatible Token class
#[pyclass(name = "Token")]
#[derive(Clone)]
pub struct PyToken {
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub start: usize,
    #[pyo3(get)]
    pub len: usize,
    #[pyo3(get)]
    pub chunk_type: String,
    #[pyo3(get)]
    pub pos: Option<String>,
    #[pyo3(get)]
    pub lemma: Option<String>,
    #[pyo3(get)]
    pub freq: Option<u32>,
    #[pyo3(get)]
    pub syls: Vec<String>,
    #[pyo3(get)]
    pub is_affix: bool,
    #[pyo3(get)]
    pub is_affix_host: bool,
    #[pyo3(get)]
    pub is_skrt: bool,
}

impl From<RustToken> for PyToken {
    fn from(t: RustToken) -> Self {
        PyToken {
            text: t.text,
            start: t.start,
            len: t.len,
            chunk_type: t.chunk_type.as_str().to_string(),
            pos: t.pos,
            lemma: t.lemma,
            freq: t.freq,
            syls: t.syls,
            is_affix: t.is_affix,
            is_affix_host: t.is_affix_host,
            is_skrt: t.is_skrt,
        }
    }
}

#[pymethods]
impl PyToken {
    fn __repr__(&self) -> String {
        if let Some(ref pos) = self.pos {
            format!("Token('{}', pos='{}')", self.text, pos)
        } else {
            format!("Token('{}', chunk_type='{}')", self.text, self.chunk_type)
        }
    }

    fn __str__(&self) -> String {
        self.text.clone()
    }

    /// Get the cleaned text (with proper tsek placement)
    fn text_cleaned(&self) -> String {
        if self.syls.is_empty() {
            return String::new();
        }
        let mut cleaned = self.syls.join("་");
        if !(self.is_affix_host && !self.is_affix) {
            cleaned.push('་');
        }
        cleaned
    }

    /// Check if this is a word token
    fn is_word(&self) -> bool {
        self.chunk_type == "TEXT" && !self.syls.is_empty()
    }

    /// Check if this is punctuation
    fn is_punct(&self) -> bool {
        self.chunk_type == "PUNCT"
    }

    /// Convert to dictionary
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("text", &self.text)?;
        dict.set_item("start", self.start)?;
        dict.set_item("len", self.len)?;
        dict.set_item("chunk_type", &self.chunk_type)?;
        dict.set_item("pos", &self.pos)?;
        dict.set_item("lemma", &self.lemma)?;
        dict.set_item("freq", self.freq)?;
        dict.set_item("syls", &self.syls)?;
        dict.set_item("is_affix", self.is_affix)?;
        dict.set_item("is_affix_host", self.is_affix_host)?;
        dict.set_item("is_skrt", self.is_skrt)?;
        Ok(dict)
    }
}

/// Word Tokenizer - the main tokenizer class
/// 
/// This provides dictionary-based tokenization using longest-match.
/// 
/// Example:
///     >>> from botok_rs import WordTokenizer
///     >>> wt = WordTokenizer()
///     >>> wt.load_tsv("བཀྲ་ཤིས\\tNOUN\\t\\t\\t1000")
///     >>> tokens = wt.tokenize("བཀྲ་ཤིས།")
///     >>> for t in tokens:
///     ...     print(t.text, t.pos)
#[pyclass(name = "WordTokenizer")]
pub struct PyWordTokenizer {
    trie: Trie,
}

#[pymethods]
impl PyWordTokenizer {
    #[new]
    fn new() -> Self {
        PyWordTokenizer { trie: Trie::new() }
    }

    /// Load words from a TSV string
    /// 
    /// Format: form\tpos\tlemma\tsense\tfreq
    /// Lines starting with # are comments.
    fn load_tsv(&mut self, tsv_content: &str) {
        let mut builder = TrieBuilder::new();
        builder.load_tsv(tsv_content);
        self.trie = builder.build();
    }

    /// Load words from a TSV file
    fn load_tsv_file(&mut self, path: &str) -> PyResult<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        self.load_tsv(&content);
        Ok(())
    }

    /// Add a single word to the dictionary
    /// 
    /// Args:
    ///     word: The word form (with tsek between syllables)
    ///     pos: Part-of-speech tag (optional)
    ///     lemma: Base form (optional)
    ///     freq: Frequency (optional)
    #[pyo3(signature = (word, pos=None, lemma=None, freq=None))]
    fn add_word(&mut self, word: &str, pos: Option<&str>, lemma: Option<&str>, freq: Option<u32>) {
        let data = WordData {
            pos: pos.map(|s| s.to_string()),
            lemma: lemma.map(|s| s.to_string()),
            freq,
            ..Default::default()
        };
        self.trie.add_word(word, Some(data));
    }

    /// Tokenize a string
    /// 
    /// Args:
    ///     text: The Tibetan text to tokenize
    /// 
    /// Returns:
    ///     List of Token objects
    fn tokenize(&self, text: &str) -> Vec<PyToken> {
        let tokenizer = RustTokenizer::new(self.trie.clone());
        tokenizer
            .tokenize(text)
            .into_iter()
            .map(PyToken::from)
            .collect()
    }

    /// Get the number of words in the dictionary
    fn __len__(&self) -> usize {
        self.trie.len()
    }

    fn __repr__(&self) -> String {
        format!("WordTokenizer(words={})", self.trie.len())
    }
}

/// Simple Tokenizer - syllable-level tokenization without dictionary
/// 
/// This tokenizer just splits text into syllables without dictionary lookup.
/// Useful for basic segmentation or when you don't have a dictionary.
/// 
/// Example:
///     >>> from botok_rs import SimpleTokenizer
///     >>> tokens = SimpleTokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
///     >>> for t in tokens:
///     ...     print(t.text)
#[pyclass(name = "SimpleTokenizer")]
pub struct PySimpleTokenizer;

#[pymethods]
impl PySimpleTokenizer {
    #[new]
    fn new() -> Self {
        PySimpleTokenizer
    }

    /// Tokenize text into syllables (no dictionary lookup)
    /// 
    /// Args:
    ///     text: The Tibetan text to tokenize
    /// 
    /// Returns:
    ///     List of Token objects
    #[staticmethod]
    fn tokenize(text: &str) -> Vec<PyToken> {
        RustSimpleTokenizer::tokenize(text)
            .into_iter()
            .map(PyToken::from)
            .collect()
    }
}

/// Chunk text into typed segments (syllables, punctuation, etc.)
/// 
/// This is a lower-level function that segments text without tokenization.
/// 
/// Args:
///     text: The text to chunk
/// 
/// Returns:
///     List of tuples: (chunk_type, text, start, len)
#[pyfunction]
fn chunk<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyList>> {
    let chunker = Chunker::new(text);
    let chunks = chunker.make_chunks();

    let list = PyList::empty_bound(py);
    for chunk in chunks {
        let tuple = (
            match chunk.chunk_type {
                ChunkType::Text => "TEXT",
                ChunkType::Punct => "PUNCT",
                ChunkType::Num => "NUM",
                ChunkType::Sym => "SYM",
                ChunkType::Latin => "LATIN",
                ChunkType::Cjk => "CJK",
                ChunkType::Other => "OTHER",
            },
            &text[chunk.start..chunk.start + chunk.len],
            chunk.start,
            chunk.len,
        );
        list.append(tuple)?;
    }

    Ok(list)
}

/// Get syllables from Tibetan text
/// 
/// Args:
///     text: The Tibetan text
/// 
/// Returns:
///     List of syllable strings (without tsek)
#[pyfunction]
fn get_syls(text: &str) -> Vec<String> {
    let chunker = Chunker::new(text);
    let chunks = chunker.make_chunks();

    chunks
        .into_iter()
        .filter_map(|c| c.syl)
        .collect()
}

/// Tokenize text using simple syllable tokenization
/// 
/// This is a convenience function equivalent to SimpleTokenizer.tokenize()
/// 
/// Args:
///     text: The Tibetan text to tokenize
/// 
/// Returns:
///     List of Token objects
#[pyfunction]
fn tokenize_simple(text: &str) -> Vec<PyToken> {
    PySimpleTokenizer::tokenize(text)
}

/// Create the Python module
#[pymodule]
fn botok_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyToken>()?;
    m.add_class::<PyWordTokenizer>()?;
    m.add_class::<PySimpleTokenizer>()?;
    m.add_function(wrap_pyfunction!(chunk, m)?)?;
    m.add_function(wrap_pyfunction!(get_syls, m)?)?;
    m.add_function(wrap_pyfunction!(tokenize_simple, m)?)?;

    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
