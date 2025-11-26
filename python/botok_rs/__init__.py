"""
botok-rs: A fast Tibetan tokenizer written in Rust

This is a high-performance Rust implementation of the botok tokenizer,
with Python bindings for easy integration.

Example usage:

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

    # Dictionary-based tokenization
    >>> from botok_rs import WordTokenizer
    >>> wt = WordTokenizer()
    >>> wt.load_tsv_file("dictionary.tsv")
    >>> tokens = wt.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
    >>> for t in tokens:
    ...     print(f"{t.text} ({t.pos})")

    # Low-level chunking
    >>> from botok_rs import chunk, get_syls
    >>> chunks = chunk("བཀྲ་ཤིས། Hello")
    >>> syls = get_syls("བཀྲ་ཤིས་བདེ་ལེགས")
"""

# Import from the Rust extension
from .botok_rs import (
    Token,
    WordTokenizer,
    SimpleTokenizer,
    chunk,
    get_syls,
    tokenize_simple,
    __version__,
)

__all__ = [
    "Token",
    "WordTokenizer", 
    "SimpleTokenizer",
    "chunk",
    "get_syls",
    "tokenize_simple",
    "__version__",
]

