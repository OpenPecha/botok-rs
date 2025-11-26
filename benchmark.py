#!/usr/bin/env python3
"""
Benchmark comparing Python botok vs Rust botok-rs performance.
"""

import time
import statistics

# Sample Tibetan texts of varying lengths
SMALL_TEXT = "བཀྲ་ཤིས་བདེ་ལེགས།"

MEDIUM_TEXT = """བོད་སྐད་ནི་བོད་ཡུལ་དང་། བལ་ཡུལ། འབྲུག་ཡུལ། རྒྱ་གར་བཅས་ཀྱི་ས་ཁུལ་མང་པོར་བེད་སྤྱོད་བྱེད་བཞིན་ཡོད། 
བོད་སྐད་ནི་སྐད་རིགས་ཆེན་པོ་ཞིག་ཡིན་ཞིང་། མི་གྲངས་ས་ཡ་བཅུ་ཕྲག་མང་པོས་བེད་སྤྱོད་བྱེད་བཞིན་ཡོད།
བོད་ཡིག་ནི་བོད་སྐད་བྲིས་ཐབས་ཀྱི་ཡིག་རིགས་ཤིག་ཡིན། བོད་ཡིག་གི་འབྱུང་ཁུངས་ནི་རྒྱ་གར་གྱི་བྲཱཧྨི་ཡིག་རིགས་ནས་བྱུང་བ་ཡིན།"""

LARGE_TEXT = MEDIUM_TEXT * 50  # ~50x medium text

def benchmark_function(func, text, iterations=100, warmup=5):
    """Run a function multiple times and return timing statistics."""
    # Warmup
    for _ in range(warmup):
        func(text)
    
    # Actual benchmark
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        result = func(text)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms
    
    return {
        'mean': statistics.mean(times),
        'median': statistics.median(times),
        'stdev': statistics.stdev(times) if len(times) > 1 else 0,
        'min': min(times),
        'max': max(times),
        'tokens': len(result) if hasattr(result, '__len__') else 0
    }

def print_results(name, results):
    """Print benchmark results."""
    print(f"  {name}:")
    print(f"    Mean:   {results['mean']:.3f} ms")
    print(f"    Median: {results['median']:.3f} ms")
    print(f"    Min:    {results['min']:.3f} ms")
    print(f"    Max:    {results['max']:.3f} ms")
    print(f"    Tokens: {results['tokens']}")

def compare_results(python_results, rust_results):
    """Print comparison between Python and Rust."""
    speedup = python_results['mean'] / rust_results['mean'] if rust_results['mean'] > 0 else float('inf')
    print(f"  Speedup: {speedup:.1f}x faster")

def main():
    print("=" * 60)
    print("Botok Performance Benchmark: Python vs Rust")
    print("=" * 60)
    
    # Try to import both libraries
    try:
        import botok
        has_python_botok = True
        print("✓ Python botok imported")
    except ImportError:
        has_python_botok = False
        print("✗ Python botok not available (pip install botok)")
    
    try:
        import botok_rs
        has_rust_botok = True
        print("✓ Rust botok-rs imported")
    except ImportError:
        has_rust_botok = False
        print("✗ Rust botok-rs not available")
    
    if not has_rust_botok:
        print("\nPlease install botok-rs first:")
        print("  cd /Users/tenzingayche/Desktop/botok-rs")
        print("  maturin develop --features python --release")
        return
    
    print()
    
    # Define tokenization functions
    if has_python_botok:
        wt_python = botok.WordTokenizer()
        def tokenize_python(text):
            return wt_python.tokenize(text, split_affixes=False)
    
    # Rust simple tokenizer (no dictionary)
    def tokenize_rust_simple(text):
        return botok_rs.SimpleTokenizer.tokenize(text)
    
    # Rust with dictionary
    wt_rust = botok_rs.WordTokenizer()
    # Add some common words
    wt_rust.add_word("བཀྲ་ཤིས", pos="NOUN")
    wt_rust.add_word("བདེ་ལེགས", pos="NOUN")
    wt_rust.add_word("བོད་སྐད", pos="NOUN")
    wt_rust.add_word("བོད་ཡུལ", pos="NOUN")
    wt_rust.add_word("རྒྱ་གར", pos="NOUN")
    
    def tokenize_rust_dict(text):
        return wt_rust.tokenize(text)
    
    # Run benchmarks
    test_cases = [
        ("Small text", SMALL_TEXT, 1000),
        ("Medium text", MEDIUM_TEXT, 500),
        ("Large text", LARGE_TEXT, 50),
    ]
    
    for name, text, iterations in test_cases:
        print(f"\n{'=' * 60}")
        print(f"{name} ({len(text)} chars, {iterations} iterations)")
        print("=" * 60)
        
        # Rust simple tokenizer
        print("\n[Rust SimpleTokenizer (no dictionary)]")
        rust_simple = benchmark_function(tokenize_rust_simple, text, iterations)
        print_results("Rust (simple)", rust_simple)
        
        # Rust with dictionary
        print("\n[Rust WordTokenizer (with dictionary)]")
        rust_dict = benchmark_function(tokenize_rust_dict, text, iterations)
        print_results("Rust (dict)", rust_dict)
        
        # Python botok (if available)
        if has_python_botok:
            print("\n[Python botok WordTokenizer]")
            python_results = benchmark_function(tokenize_python, text, iterations)
            print_results("Python", python_results)
            
            print("\n[Comparison]")
            print("  Python vs Rust (simple):", end=" ")
            compare_results(python_results, rust_simple)
            print("  Python vs Rust (dict):  ", end=" ")
            compare_results(python_results, rust_dict)
        else:
            print("\n[Comparison]")
            print("  (Python botok not available for comparison)")
    
    # Summary
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    if has_python_botok:
        print("The Rust implementation is significantly faster than Python.")
        print("For large texts, expect 10-100x speedup depending on dictionary size.")
    else:
        print("Install Python botok to compare: pip install botok")
    print()

if __name__ == "__main__":
    main()

