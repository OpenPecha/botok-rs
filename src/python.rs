//! Python bindings for botok-rs using PyO3
//!
//! This module provides Python-compatible wrappers around the Rust tokenizer.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::chunker::Chunker;
use crate::token::{ChunkType, Token as RustToken};
use crate::tokenizer::{SimpleTokenizer as RustSimpleTokenizer, Tokenizer as RustTokenizer};
use crate::trie::{Trie, TrieBuilder, WordData};

#[cfg(feature = "download")]
use crate::dialect_pack;

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
/// By default, it automatically downloads the dialect pack on first use.
/// 
/// Example:
///     >>> from botok_rs import WordTokenizer
///     >>> wt = WordTokenizer()  # Auto-downloads dialect pack
///     >>> tokens = wt.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
///     >>> for t in tokens:
///     ...     print(t.text, t.pos)
#[pyclass(name = "WordTokenizer")]
pub struct PyWordTokenizer {
    /// Shared trie reference - avoids expensive clones on each tokenize() call
    trie: Arc<Trie>,
}

#[pymethods]
impl PyWordTokenizer {
    /// Create a new WordTokenizer.
    /// 
    /// Args:
    ///     dialect_name: Name of the dialect pack to use (default: "general")
    ///     base_path: Base path for dialect packs (default: ~/Documents/botok-rs/dialect_packs/)
    ///     auto_download: Whether to automatically download the dialect pack (default: True)
    /// 
    /// If auto_download is True and the dialect pack is not found locally,
    /// it will be downloaded from GitHub automatically.
    #[new]
    #[pyo3(signature = (dialect_name=None, base_path=None, auto_download=true))]
    fn new(dialect_name: Option<&str>, base_path: Option<&str>, auto_download: bool) -> PyResult<Self> {
        let mut trie = Trie::new();
        
        #[cfg(feature = "download")]
        if auto_download {
            let dialect = dialect_name.unwrap_or(dialect_pack::DEFAULT_DIALECT_PACK);
            let base = base_path.map(std::path::Path::new);
            
            // Download dialect pack if needed
            let pack_path = dialect_pack::get_dialect_pack(dialect, base)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            // Load all dictionary files
            let dict_files = dialect_pack::list_dictionary_files(&pack_path)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
            
            let mut builder = TrieBuilder::new();
            for file in dict_files {
                if let Ok(content) = std::fs::read_to_string(&file) {
                    builder.load_tsv(&content);
                }
            }
            trie = builder.build();
        }
        
        Ok(PyWordTokenizer { trie: Arc::new(trie) })
    }

    /// Load words from a TSV string
    /// 
    /// Format: form\tpos\tlemma\tsense\tfreq
    /// Lines starting with # are comments.
    fn load_tsv(&mut self, tsv_content: &str) {
        let mut builder = TrieBuilder::new();
        builder.load_tsv(tsv_content);
        // Merge with existing trie - need to get mutable access
        let new_trie = builder.build();
        let mut trie = (*self.trie).clone();
        trie.merge(&new_trie);
        self.trie = Arc::new(trie);
    }

    /// Load words from a TSV file
    fn load_tsv_file(&mut self, path: &str) -> PyResult<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        self.load_tsv(&content);
        Ok(())
    }

    /// Load a dialect pack by name
    /// 
    /// Args:
    ///     dialect_name: Name of the dialect pack (e.g., "general")
    ///     base_path: Base path for dialect packs (optional)
    /// 
    /// This will download the dialect pack if not already present.
    #[cfg(feature = "download")]
    #[pyo3(signature = (dialect_name, base_path=None))]
    fn load_dialect_pack(&mut self, dialect_name: &str, base_path: Option<&str>) -> PyResult<()> {
        let base = base_path.map(std::path::Path::new);
        
        let pack_path = dialect_pack::get_dialect_pack(dialect_name, base)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        let dict_files = dialect_pack::list_dictionary_files(&pack_path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        let mut builder = TrieBuilder::new();
        for file in dict_files {
            if let Ok(content) = std::fs::read_to_string(&file) {
                builder.load_tsv(&content);
            }
        }
        self.trie = Arc::new(builder.build());
        
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
        // Clone the trie, modify it, and replace
        let mut trie = (*self.trie).clone();
        trie.add_word(word, Some(data));
        self.trie = Arc::new(trie);
    }

    /// Tokenize a string
    /// 
    /// Args:
    ///     text: The Tibetan text to tokenize
    ///     split_affixes: Whether to split affixed particles (default: True)
    /// 
    /// Returns:
    ///     List of Token objects
    #[pyo3(signature = (text, split_affixes=true))]
    fn tokenize(&self, text: &str, split_affixes: bool) -> Vec<PyToken> {
        // Use Arc::clone for cheap reference counting instead of cloning the whole trie
        let tokenizer = RustTokenizer::with_arc(Arc::clone(&self.trie));
        tokenizer
            .tokenize_with_options(text, split_affixes)
            .into_iter()
            .map(PyToken::from)
            .collect()
    }

    /// Get the number of words in the dictionary
    fn __len__(&self) -> usize {
        (*self.trie).len()
    }

    fn __repr__(&self) -> String {
        format!("WordTokenizer(words={})", (*self.trie).len())
    }
}

/// Trie data structure wrapper for Python
/// 
/// This wraps the internal Trie for advanced usage.
#[pyclass(name = "Trie")]
pub struct PyTrie {
    trie: Trie,
}

#[pymethods]
impl PyTrie {
    /// Get the number of words in the trie
    fn __len__(&self) -> usize {
        self.trie.len()
    }

    /// Check if a word exists in the trie
    fn has_word(&self, word: &str) -> bool {
        let syls: Vec<&str> = word.split('་').filter(|s| !s.is_empty()).collect();
        self.trie.has_word(&syls)
    }

    fn __repr__(&self) -> String {
        format!("Trie(words={})", self.trie.len())
    }
}

/// Trie Builder - for building custom dictionaries
/// 
/// Example:
///     >>> from botok_rs import TrieBuilder
///     >>> builder = TrieBuilder()  # Without inflection
///     >>> builder = TrieBuilder.with_inflection()  # With auto-inflection
///     >>> builder.load_tsv("བཀྲ་ཤིས\tNOUN\t\t\t1000")
///     >>> trie = builder.build()
#[pyclass(name = "TrieBuilder")]
pub struct PyTrieBuilder {
    builder: TrieBuilder,
}

#[pymethods]
impl PyTrieBuilder {
    /// Create a new TrieBuilder without inflection
    #[new]
    fn new() -> Self {
        PyTrieBuilder {
            builder: TrieBuilder::new(),
        }
    }

    /// Create a new TrieBuilder with auto-inflection enabled
    /// 
    /// When inflection is enabled, all affixed forms of each word
    /// are automatically generated and added to the trie.
    #[staticmethod]
    fn with_inflection() -> Self {
        PyTrieBuilder {
            builder: TrieBuilder::with_inflection(),
        }
    }

    /// Enable or disable auto-inflection
    fn set_inflection(&mut self, enable: bool) {
        self.builder.set_inflection(enable);
    }

    /// Load words from a TSV string
    /// 
    /// Format: form\tpos\tlemma\tsense\tfreq
    /// Lines starting with # are comments.
    fn load_tsv(&mut self, tsv_content: &str) {
        self.builder.load_tsv(tsv_content);
    }

    /// Load words from a TSV file
    fn load_tsv_file(&mut self, path: &str) -> PyResult<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        self.builder.load_tsv(&content);
        Ok(())
    }

    /// Add a word with all its inflected forms (if inflection is enabled)
    #[pyo3(signature = (word, pos=None, lemma=None, freq=None))]
    fn add_word(&mut self, word: &str, pos: Option<&str>, lemma: Option<&str>, freq: Option<u32>) {
        let data = WordData {
            pos: pos.map(|s| s.to_string()),
            lemma: lemma.map(|s| s.to_string()),
            freq,
            ..Default::default()
        };
        self.builder.add_inflected_word(word, Some(data));
    }

    /// Deactivate a word and all its inflected forms
    fn deactivate_word(&mut self, word: &str) {
        self.builder.deactivate_inflected_word(word);
    }

    /// Build and return the Trie
    /// 
    /// Note: This consumes the builder. Create a new builder for additional tries.
    fn build(&mut self) -> PyTrie {
        // We need to swap out the builder since we can't consume self in PyO3
        let builder = std::mem::replace(&mut self.builder, TrieBuilder::new());
        PyTrie {
            trie: builder.build(),
        }
    }

    /// Get the current number of words in the trie being built
    fn __len__(&self) -> usize {
        self.builder.trie().len()
    }

    fn __repr__(&self) -> String {
        format!("TrieBuilder(words={})", self.builder.trie().len())
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

/// Download a dialect pack from GitHub
/// 
/// Args:
///     dialect_name: Name of the dialect pack (default: "general")
///     base_path: Base path for dialect packs (optional)
///     version: Specific version to download (optional, defaults to latest)
/// 
/// Returns:
///     Path to the downloaded dialect pack
#[cfg(feature = "download")]
#[pyfunction]
#[pyo3(signature = (dialect_name=None, base_path=None, version=None))]
fn download_dialect_pack(
    dialect_name: Option<&str>,
    base_path: Option<&str>,
    version: Option<&str>,
) -> PyResult<String> {
    let dialect = dialect_name.unwrap_or(dialect_pack::DEFAULT_DIALECT_PACK);
    let base = base_path.map(std::path::Path::new);
    
    let path = dialect_pack::download_dialect_pack(dialect, base, version)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    Ok(path.to_string_lossy().to_string())
}

/// Get the path to a dialect pack
/// 
/// Args:
///     dialect_name: Name of the dialect pack (default: "general")
///     base_path: Base path for dialect packs (optional)
/// 
/// Returns:
///     Path to the dialect pack directory
#[cfg(feature = "download")]
#[pyfunction]
#[pyo3(signature = (dialect_name=None, base_path=None))]
fn get_dialect_pack_path(dialect_name: Option<&str>, base_path: Option<&str>) -> String {
    let dialect = dialect_name.unwrap_or(dialect_pack::DEFAULT_DIALECT_PACK);
    let base = base_path.map(std::path::Path::new);
    dialect_pack::dialect_pack_path(dialect, base).to_string_lossy().to_string()
}

/// Check if a dialect pack exists locally
/// 
/// Args:
///     dialect_name: Name of the dialect pack (default: "general")
///     base_path: Base path for dialect packs (optional)
/// 
/// Returns:
///     True if the dialect pack exists locally
#[cfg(feature = "download")]
#[pyfunction]
#[pyo3(signature = (dialect_name=None, base_path=None))]
fn dialect_pack_exists(dialect_name: Option<&str>, base_path: Option<&str>) -> bool {
    let dialect = dialect_name.unwrap_or(dialect_pack::DEFAULT_DIALECT_PACK);
    let base = base_path.map(std::path::Path::new);
    dialect_pack::dialect_pack_exists(dialect, base)
}

/// Get the default base path for dialect packs
/// 
/// Returns:
///     Default path where dialect packs are stored (~/Documents/botok-rs/dialect_packs/)
#[cfg(feature = "download")]
#[pyfunction]
fn get_default_base_path() -> String {
    dialect_pack::default_base_path().to_string_lossy().to_string()
}

/// Create the Python module
#[pymodule]
fn botok_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyToken>()?;
    m.add_class::<PyWordTokenizer>()?;
    m.add_class::<PySimpleTokenizer>()?;
    m.add_class::<PyTrie>()?;
    m.add_class::<PyTrieBuilder>()?;
    m.add_function(wrap_pyfunction!(chunk, m)?)?;
    m.add_function(wrap_pyfunction!(get_syls, m)?)?;
    m.add_function(wrap_pyfunction!(tokenize_simple, m)?)?;
    
    // Dialect pack functions (only available with download feature)
    #[cfg(feature = "download")]
    {
        m.add_function(wrap_pyfunction!(download_dialect_pack, m)?)?;
        m.add_function(wrap_pyfunction!(get_dialect_pack_path, m)?)?;
        m.add_function(wrap_pyfunction!(dialect_pack_exists, m)?)?;
        m.add_function(wrap_pyfunction!(get_default_base_path, m)?)?;
    }

    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
