//! Sentence and paragraph tokenization.
//!
//! This module provides higher-level tokenization that groups word tokens
//! into sentences and paragraphs based on Tibetan punctuation and grammar.

use crate::token::{ChunkType, Token};

/// Ending particles that typically mark sentence boundaries
static ENDING_PARTICLES: &[&str] = &[
    "གོ་", "ངོ་", "དོ་", "ནོ་", "བོ་", "མོ་", "འོ་", "རོ་", "ལོ་", "སོ་", "ཏོ་",
];

/// Words that typically end sentences
static ENDING_WORDS: &[&str] = &["ཅིག་", "ཤོག་"];

/// Verbs that often mark sentence boundaries
static ENDING_VERBS: &[&str] = &[
    "ཡིན་", "ཡོད་", "མིན་", "མེད་", "འགྱུར་", "ལྡན་", "བགྱི་", "བྱ་", "བཞུགས་", "འདུག་", "སོང་",
];

/// Clause boundary particles
static CLAUSE_BOUNDARIES: &[&str] = &["སྟེ་", "ཏེ་", "དེ་", "ནས་", "ན་", "ལ་", "ཞིང་"];

/// Dagdra particles (pa/po/ba/bo)
static DAGDRA: &[&str] = &["པ་", "བ་", "པོ་", "བོ་"];

/// A sentence containing tokens and metadata
#[derive(Debug, Clone)]
pub struct Sentence {
    /// The tokens in this sentence
    pub tokens: Vec<Token>,
    /// Number of word tokens (excluding punctuation)
    pub word_count: usize,
    /// Start index in the original token list
    pub start_idx: usize,
    /// End index in the original token list (inclusive)
    pub end_idx: usize,
}

impl Sentence {
    /// Get the text of this sentence
    pub fn text(&self) -> String {
        self.tokens.iter().map(|t| t.text.as_str()).collect()
    }

    /// Get the normalized text of this sentence
    pub fn normalized_text(&self) -> String {
        let text = self.text();
        // Apply basic normalization
        text.replace("༑", "།")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A paragraph containing sentences
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// The sentences in this paragraph
    pub sentences: Vec<Sentence>,
    /// Total word count
    pub word_count: usize,
}

impl Paragraph {
    /// Get the text of this paragraph
    pub fn text(&self) -> String {
        self.sentences.iter().map(|s| s.text()).collect()
    }
}

/// Tokenize a list of word tokens into sentences
pub fn sentence_tokenize(tokens: &[Token]) -> Vec<Sentence> {
    if tokens.is_empty() {
        return vec![];
    }

    let indices = get_sentence_indices(tokens);
    
    indices
        .into_iter()
        .map(|(start, end)| {
            let sentence_tokens = tokens[start..=end].to_vec();
            let word_count = sentence_tokens
                .iter()
                .filter(|t| t.chunk_type == ChunkType::Text)
                .count();
            
            Sentence {
                tokens: sentence_tokens,
                word_count,
                start_idx: start,
                end_idx: end,
            }
        })
        .collect()
}

/// Tokenize a list of word tokens into paragraphs
pub fn paragraph_tokenize(tokens: &[Token]) -> Vec<Paragraph> {
    let sentences = sentence_tokenize(tokens);
    
    if sentences.is_empty() {
        return vec![];
    }

    let threshold = 70;
    let paragraph_max = 150;

    let mut paragraphs: Vec<Paragraph> = Vec::new();
    let mut current_sentences: Vec<Sentence> = Vec::new();
    let mut current_word_count = 0;

    for sentence in sentences {
        let sentence_words = sentence.word_count;
        
        if current_word_count + sentence_words > paragraph_max && !current_sentences.is_empty() {
            // Start a new paragraph
            paragraphs.push(Paragraph {
                sentences: std::mem::take(&mut current_sentences),
                word_count: current_word_count,
            });
            current_word_count = 0;
        }
        
        current_word_count += sentence_words;
        current_sentences.push(sentence);
        
        // If we have enough words, consider it a paragraph
        if current_word_count >= threshold {
            paragraphs.push(Paragraph {
                sentences: std::mem::take(&mut current_sentences),
                word_count: current_word_count,
            });
            current_word_count = 0;
        }
    }

    // Don't forget the last paragraph
    if !current_sentences.is_empty() {
        paragraphs.push(Paragraph {
            sentences: current_sentences,
            word_count: current_word_count,
        });
    }

    paragraphs
}

/// Get sentence boundary indices
fn get_sentence_indices(tokens: &[Token]) -> Vec<(usize, usize)> {
    if tokens.is_empty() {
        return vec![];
    }

    // Step 1: Find unambiguous sentence boundaries (ending particle + punctuation)
    let mut boundaries = find_boundaries(tokens, is_ending_particle_and_punct);

    // Step 2: Find clause boundaries followed by punctuation
    boundaries = refine_boundaries(tokens, &boundaries, is_clause_boundary_and_punct);

    // Step 3: Find verbs followed by punctuation
    boundaries = refine_boundaries(tokens, &boundaries, is_verb_and_punct);

    // Step 4: Find verbs followed by clause boundaries (for long sentences)
    boundaries = refine_long_sentences(tokens, &boundaries, is_verb_and_clause_boundary, 30);

    // Step 5: Join short sentences without verbs
    boundaries = join_no_verb_sentences(tokens, &boundaries, 4);

    boundaries
}

/// Find initial sentence boundaries based on a test function
fn find_boundaries<F>(tokens: &[Token], test: F) -> Vec<(usize, usize)>
where
    F: Fn(&Token, &Token) -> bool,
{
    let mut boundaries = Vec::new();
    let mut start = 0;

    for i in 1..tokens.len() {
        if test(&tokens[i - 1], &tokens[i]) {
            boundaries.push((start, i));
            start = i + 1;
        }
    }

    // Add the last segment
    if start < tokens.len() {
        boundaries.push((start, tokens.len() - 1));
    }

    if boundaries.is_empty() {
        boundaries.push((0, tokens.len() - 1));
    }

    boundaries
}

/// Refine boundaries by splitting long segments
fn refine_boundaries<F>(tokens: &[Token], boundaries: &[(usize, usize)], test: F) -> Vec<(usize, usize)>
where
    F: Fn(&Token, &Token) -> bool,
{
    let mut result = Vec::new();

    for &(start, end) in boundaries {
        let mut segment_start = start;
        
        for i in (start + 1)..=end {
            if i < tokens.len() && test(&tokens[i - 1], &tokens[i]) {
                result.push((segment_start, i));
                segment_start = i + 1;
            }
        }
        
        if segment_start <= end {
            result.push((segment_start, end));
        }
    }

    result
}

/// Refine only long sentences
fn refine_long_sentences<F>(
    tokens: &[Token],
    boundaries: &[(usize, usize)],
    test: F,
    threshold: usize,
) -> Vec<(usize, usize)>
where
    F: Fn(&Token, &Token) -> bool,
{
    let mut result = Vec::new();

    for &(start, end) in boundaries {
        if end - start > threshold {
            // This segment is long, try to split it
            let mut segment_start = start;
            
            for i in (start + 1)..=end {
                if i < tokens.len() && test(&tokens[i - 1], &tokens[i]) {
                    result.push((segment_start, i));
                    segment_start = i + 1;
                }
            }
            
            if segment_start <= end {
                result.push((segment_start, end));
            }
        } else {
            result.push((start, end));
        }
    }

    result
}

/// Join short sentences without verbs to adjacent sentences
fn join_no_verb_sentences(
    tokens: &[Token],
    boundaries: &[(usize, usize)],
    threshold: usize,
) -> Vec<(usize, usize)> {
    let mut result: Vec<(usize, usize)> = boundaries.to_vec();
    let mut i = 0;

    while i < result.len() {
        let (start, end) = result[i];
        let length = end - start + 1;

        if length <= threshold {
            // Check if this segment has a verb
            let has_verb = tokens[start..=end]
                .iter()
                .any(|t| t.pos.as_deref() == Some("VERB") && !has_last_syl(t, DAGDRA));

            if !has_verb {
                // Try to join with adjacent segment
                if i + 1 < result.len() && has_last_syl(&tokens[end], CLAUSE_BOUNDARIES) {
                    // Join with next
                    result[i + 1].0 = start;
                    result.remove(i);
                    continue;
                } else if i > 0 {
                    // Join with previous
                    let prev_end = result[i - 1].1;
                    if !has_last_syl(&tokens[prev_end], ENDING_PARTICLES) {
                        result[i - 1].1 = end;
                        result.remove(i);
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    result
}

// Helper functions for sentence boundary detection

#[allow(dead_code)]
fn is_word(token: &Token) -> bool {
    token.chunk_type == ChunkType::Text
}

fn has_last_syl(token: &Token, patterns: &[&str]) -> bool {
    if token.syls.is_empty() {
        return false;
    }
    
    let last_syl = format!("{}་", token.syls.last().unwrap());
    patterns.iter().any(|p| last_syl == *p)
}

fn is_ending_particle(token: &Token) -> bool {
    token.pos.as_deref() == Some("PART") && has_last_syl(token, ENDING_PARTICLES)
}

fn is_ending_particle_and_punct(token1: &Token, token2: &Token) -> bool {
    is_ending_particle(token1) && token2.chunk_type == ChunkType::Punct
}

fn is_clause_boundary_and_punct(token1: &Token, token2: &Token) -> bool {
    (has_last_syl(token1, CLAUSE_BOUNDARIES) || has_last_syl(token1, ENDING_WORDS))
        && token2.chunk_type == ChunkType::Punct
}

fn is_verb_and_punct(token1: &Token, token2: &Token) -> bool {
    let is_verb = (token1.pos.as_deref() == Some("VERB") && !has_last_syl(token1, DAGDRA))
        || has_last_syl(token1, ENDING_VERBS);
    is_verb && token2.chunk_type == ChunkType::Punct
}

fn is_verb_and_clause_boundary(token1: &Token, token2: &Token) -> bool {
    let is_verb = (token1.pos.as_deref() == Some("VERB") && !has_last_syl(token1, DAGDRA))
        || has_last_syl(token1, ENDING_VERBS);
    is_verb && has_last_syl(token2, CLAUSE_BOUNDARIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_token(text: &str, chunk_type: ChunkType, pos: Option<&str>) -> Token {
        let mut token = Token::with_text(text.to_string(), 0, text.len(), chunk_type);
        token.pos = pos.map(|s| s.to_string());
        // Extract syllables from text
        token.syls = text.split('་')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        token
    }

    #[test]
    fn test_sentence_tokenize_basic() {
        let tokens = vec![
            make_token("བཀྲ་ཤིས་", ChunkType::Text, Some("NOUN")),
            make_token("བདེ་ལེགས་", ChunkType::Text, Some("NOUN")),
            make_token("།", ChunkType::Punct, None),
            make_token("ཡིན་", ChunkType::Text, Some("VERB")),
            make_token("སོ་", ChunkType::Text, Some("PART")),
            make_token("།", ChunkType::Punct, None),
        ];

        let sentences = sentence_tokenize(&tokens);
        assert!(!sentences.is_empty());
    }

    #[test]
    fn test_paragraph_tokenize() {
        let tokens = vec![
            make_token("བཀྲ་ཤིས་", ChunkType::Text, Some("NOUN")),
            make_token("།", ChunkType::Punct, None),
        ];

        let paragraphs = paragraph_tokenize(&tokens);
        assert!(!paragraphs.is_empty());
        assert!(!paragraphs[0].sentences.is_empty());
    }

    #[test]
    fn test_has_last_syl() {
        let token = make_token("ཡིན་སོ་", ChunkType::Text, Some("PART"));
        assert!(has_last_syl(&token, ENDING_PARTICLES));
        
        let token2 = make_token("བཀྲ་ཤིས་", ChunkType::Text, Some("NOUN"));
        assert!(!has_last_syl(&token2, ENDING_PARTICLES));
    }
}

