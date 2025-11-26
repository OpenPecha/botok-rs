# botok-rs

A fast Tibetan tokenizer written in Rust.

This is a Rust port of the Python [botok](https://github.com/OpenPecha/botok) library. It provides efficient tokenization of Tibetan text using a dictionary-based longest-match algorithm.

## Features

- **Fast**: Written in Rust for maximum performance (10-100x faster than Python)
- **Dictionary-based**: Uses a Trie data structure for efficient longest-match tokenization
- **Simple mode**: Can also do basic syllable tokenization without a dictionary
- **CLI tool**: Includes a command-line interface for quick tokenization
- **Python bindings**: Use from Python with the same performance benefits
- **Library**: Can be used as a library in other Rust projects

## Installation

### Python (recommended)

```bash
pip install botok-rs
```

Or build from source:

```bash
cd botok-rs
pip install maturin
maturin build --features python --release
pip install target/wheels/botok_rs-*.whl
```

### Rust CLI

```bash
cd botok-rs
cargo build --release
```

The binary will be at `target/release/botok`.

## Usage

### Python

```python
from botok_rs import WordTokenizer

# Just works! Auto-downloads dialect pack on first use
wt = WordTokenizer()  # Downloads "general" dialect pack automatically
tokens = wt.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
for t in tokens:
    print(f"{t.text}: {t.pos}")
# བཀྲ་ཤིས་: NOUN
# བདེ་ལེགས: NOUN
# །: None
```

### Dialect Pack Management

```python
from botok_rs import (
    WordTokenizer, 
    download_dialect_pack, 
    dialect_pack_exists,
    get_default_base_path
)

# Check where dialect packs are stored
print(get_default_base_path())
# ~/Documents/botok-rs/dialect_packs/

# Check if a dialect pack exists locally
if not dialect_pack_exists("general"):
    download_dialect_pack("general")

# Use a specific dialect pack
wt = WordTokenizer(dialect_name="general")

# Disable auto-download for manual dictionary management
wt = WordTokenizer(auto_download=False)
wt.load_tsv_file("my_dictionary.tsv")
wt.add_word("བཀྲ་ཤིས", pos="NOUN")
```

### Simple Tokenization (No Dictionary)

```python
from botok_rs import SimpleTokenizer, chunk, get_syls

# Simple syllable tokenization (no dictionary)
tokens = SimpleTokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།")
for t in tokens:
    print(f"{t.text} ({t.chunk_type})")

# Get syllables
syls = get_syls("བཀྲ་ཤིས་བདེ་ལེགས")
# ['བཀྲ', 'ཤིས', 'བདེ', 'ལེགས']

# Low-level chunking
chunks = chunk("བཀྲ་ཤིས། Hello 123")
# [('TEXT', 'བཀྲ་', 0, 12), ('TEXT', 'ཤིས', 12, 9), ('PUNCT', '། ', 21, 4), ('LATIN', 'Hello 123', 25, 9)]
```

### Token Properties

```python
from botok_rs import WordTokenizer

wt = WordTokenizer()
wt.add_word("བཀྲ་ཤིས", pos="NOUN", lemma="བཀྲ་ཤིས", freq=1000)

tokens = wt.tokenize("བཀྲ་ཤིས།")
t = tokens[0]

print(t.text)        # 'བཀྲ་ཤིས་'
print(t.pos)         # 'NOUN'
print(t.lemma)       # 'བཀྲ་ཤིས'
print(t.freq)        # 1000
print(t.syls)        # ['བཀྲ', 'ཤིས']
print(t.chunk_type)  # 'TEXT'
print(t.start)       # 0 (byte offset)
print(t.len)         # 21 (byte length)

# Convenience methods
print(t.is_word())   # True
print(t.is_punct())  # False
print(t.to_dict())   # {'text': 'བཀྲ་ཤིས་', 'pos': 'NOUN', ...}
```

### Command Line

```bash
# Simple syllable tokenization (no dictionary)
botok -s "བཀྲ་ཤིས་བདེ་ལེགས།"

# With a dictionary
botok -d dictionary.tsv "བཀྲ་ཤིས་བདེ་ལེགས།"

# JSON output
botok -s -j "བཀྲ་ཤིས་བདེ་ལེགས།"

# From stdin
echo "བཀྲ་ཤིས་བདེ་ལེགས།" | botok -s
```

### As a Rust Library

```rust
use botok_rs::{Tokenizer, TrieBuilder, SimpleTokenizer};

// Option 1: Simple syllable tokenization (no dictionary)
let tokens = SimpleTokenizer::tokenize("བཀྲ་ཤིས་བདེ་ལེགས།");
for token in &tokens {
    println!("{}", token.text);
}

// Option 2: Dictionary-based tokenization
let tsv = "བཀྲ་ཤིས\tNOUN\t\t\t1000\nབདེ་ལེགས\tNOUN\t\t\t500";
let mut builder = TrieBuilder::new();
builder.load_tsv(tsv);
let trie = builder.build();

let tokenizer = Tokenizer::new(trie);
let tokens = tokenizer.tokenize("བཀྲ་ཤིས་བདེ་ལེགས།");

for token in &tokens {
    println!("{}: {:?}", token.text, token.pos);
}
```

## Dictionary Format

The dictionary uses a TSV (tab-separated values) format:

```
form	pos	lemma	sense	freq
བཀྲ་ཤིས	NOUN			1000
བདེ་ལེགས	NOUN			500
```

Columns:
- `form`: The word form (with tsek between syllables)
- `pos`: Part-of-speech tag (NOUN, VERB, etc.)
- `lemma`: Base form of the word (optional)
- `sense`: Meaning/gloss (optional)
- `freq`: Frequency count (optional)

Lines starting with `#` are treated as comments.

## Architecture

The tokenizer works in several stages:

1. **Character Classification**: Each character is classified (consonant, vowel, punctuation, etc.)
2. **Chunking**: Text is segmented into chunks (syllables, punctuation, etc.)
3. **Trie Lookup**: Syllable sequences are matched against the dictionary using longest-match
4. **Token Creation**: Matched sequences become tokens with linguistic information

## Performance

The Rust implementation is **200-400x faster** than Python botok:

| Text Size | Python botok | Rust botok-rs | Speedup |
|-----------|--------------|---------------|---------|
| **Small** (17 chars) | 0.60 ms | 0.003 ms | **240x** |
| **Medium** (306 chars) | 5.4 ms | 0.028 ms | **190x** |
| **Large** (15K chars) | 307 ms | 1.19 ms | **258x** |

### Benchmark Details

```
Small text (17 chars, 1000 iterations)
  Python:  0.603 ms (mean)
  Rust:    0.003 ms (mean)
  Speedup: 240x faster

Medium text (306 chars, 500 iterations)
  Python:  5.411 ms (mean)
  Rust:    0.028 ms (mean)
  Speedup: 190x faster

Large text (15K chars, 50 iterations)
  Python:  306.891 ms (mean)
  Rust:    1.190 ms (mean)
  Speedup: 258x faster
```

Run the benchmark yourself:

```bash
python benchmark.py
```

### Why So Fast?

- **Zero-copy parsing**: Rust's ownership model allows efficient string handling
- **Compiled code**: No interpreter overhead
- **Cache-friendly**: Trie data structure optimized for CPU cache
- **No GC pauses**: Deterministic memory management

## License

Apache-2.0 (same as the original botok)
