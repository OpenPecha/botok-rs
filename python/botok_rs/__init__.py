"""
botok-rs: A fast Tibetan tokenizer written in Rust

This is a high-performance Rust implementation of the botok tokenizer,
with Python bindings for easy integration.

Example usage:

    # Simple usage - auto-downloads dialect pack on first use!
    >>> from botok_rs import WordTokenizer
    >>> wt = WordTokenizer()  # Downloads "general" dialect pack automatically
    >>> tokens = wt.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
    >>> for t in tokens:
    ...     print(f"{t.text} ({t.pos})")
    བཀྲ་ཤིས་ (NOUN)
    བདེ་ལེགས (NOUN)
    ། (None)

    # Simple syllable tokenization (no dictionary)
    >>> from botok_rs import SimpleTokenizer
    >>> tokens = SimpleTokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
    >>> for t in tokens:
    ...     print(t.text)
    བཀྲ་
    ཤིས་
    བདེ་
    ལེགས
    །

    # Manual dictionary loading
    >>> from botok_rs import WordTokenizer
    >>> wt = WordTokenizer(auto_download=False)  # Don't auto-download
    >>> wt.load_tsv_file("dictionary.tsv")
    >>> tokens = wt.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")

    # Dialect pack management
    >>> from botok_rs import download_dialect_pack, dialect_pack_exists
    >>> if not dialect_pack_exists("general"):
    ...     download_dialect_pack("general")
    >>> wt = WordTokenizer("general")

    # Low-level chunking
    >>> from botok_rs import chunk, get_syls
    >>> chunks = chunk("བཀྲ་ཤིས། Hello")
    >>> syls = get_syls("བཀྲ་ཤིས་བདེ་ལེགས")
"""

# Import from the Rust extension
from .botok_rs import (
    # Core classes
    Token,
    WordTokenizer,
    SimpleTokenizer,
    Trie,
    TrieBuilder,
    # Functions
    chunk,
    get_syls,
    tokenize_simple,
    # Dialect pack functions
    download_dialect_pack,
    get_dialect_pack_path,
    dialect_pack_exists,
    get_default_base_path,
    # Version
    __version__,
)

__all__ = [
    # Core classes
    "Token",
    "WordTokenizer", 
    "SimpleTokenizer",
    "Trie",
    "TrieBuilder",
    # Functions
    "chunk",
    "get_syls",
    "tokenize_simple",
    # Dialect pack functions
    "download_dialect_pack",
    "get_dialect_pack_path",
    "dialect_pack_exists",
    "get_default_base_path",
    # Version
    "__version__",
]
