//! Command-line interface for botok-rs
//!
//! Usage:
//!   botok [OPTIONS] <TEXT>
//!   echo "བཀྲ་ཤིས་བདེ་ལེགས།" | botok
//!
//! Options:
//!   -d, --dict <FILE>  Path to dictionary TSV file
//!   -s, --simple       Use simple syllable tokenization (no dictionary)
//!   -j, --json         Output as JSON
//!   -h, --help         Show help

use botok_rs::{SimpleTokenizer, Tokenizer, TrieBuilder};
use std::env;
use std::fs;
use std::io::{self, BufRead};

fn print_help() {
    eprintln!(
        r#"botok-rs - A fast Tibetan tokenizer

USAGE:
    botok [OPTIONS] [TEXT]
    echo "བཀྲ་ཤིས་བདེ་ལེགས།" | botok

OPTIONS:
    -d, --dict <FILE>  Path to dictionary TSV file
    -s, --simple       Use simple syllable tokenization (no dictionary)
    -j, --json         Output as JSON
    -h, --help         Show this help message

EXAMPLES:
    botok "བཀྲ་ཤིས་བདེ་ལེགས།"
    botok -s "བཀྲ་ཤིས་བདེ་ལེགས།"
    botok -d dictionary.tsv "བཀྲ་ཤིས་བདེ་ལེགས།"
    echo "བཀྲ་ཤིས་བདེ་ལེགས།" | botok -s
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut dict_path: Option<String> = None;
    let mut simple_mode = false;
    let mut json_output = false;
    let mut text: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-s" | "--simple" => {
                simple_mode = true;
            }
            "-j" | "--json" => {
                json_output = true;
            }
            "-d" | "--dict" => {
                i += 1;
                if i < args.len() {
                    dict_path = Some(args[i].clone());
                } else {
                    eprintln!("Error: --dict requires a file path");
                    std::process::exit(1);
                }
            }
            arg if !arg.starts_with('-') => {
                text = Some(arg.to_string());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Read from stdin if no text provided
    let input_text = if let Some(t) = text {
        t
    } else {
        let stdin = io::stdin();
        let mut lines = Vec::new();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => lines.push(l),
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
        lines.join("\n")
    };

    if input_text.is_empty() {
        eprintln!("Error: No input text provided");
        print_help();
        std::process::exit(1);
    }

    // Tokenize
    let tokens = if simple_mode {
        SimpleTokenizer::tokenize(&input_text)
    } else if let Some(path) = dict_path {
        // Load dictionary and tokenize
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading dictionary file '{}': {}", path, e);
                std::process::exit(1);
            }
        };

        let mut builder = TrieBuilder::new();
        builder.load_tsv(&content);
        let trie = builder.build();
        let tokenizer = Tokenizer::new(trie);
        tokenizer.tokenize(&input_text)
    } else {
        // No dictionary - use simple mode
        SimpleTokenizer::tokenize(&input_text)
    };

    // Output
    if json_output {
        match serde_json::to_string_pretty(&tokens) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("Error serializing to JSON: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        for token in &tokens {
            if let Some(ref pos) = token.pos {
                println!("{}\t{}\t{}", token.text, pos, token.syls.join("་"));
            } else {
                println!(
                    "{}\t{}\t{}",
                    token.text,
                    token.chunk_type.as_str(),
                    token.syls.join("་")
                );
            }
        }
    }
}

