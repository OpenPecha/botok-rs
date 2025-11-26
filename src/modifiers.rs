//! Token post-processing and modification.
//!
//! This module provides functions to modify tokens after initial tokenization,
//! including splitting affixed particles, merging dagdra, and generating lemmas.

use crate::syllable::{is_dagdra, TSEK};
use crate::token::{ChunkType, Token};

/// Split tokens that contain affixed particles.
///
/// This modifies the token list in place, splitting tokens where the last
/// syllable is an affixed particle (e.g., "བཀྲ་ཤིས་ཀྱིས" -> "བཀྲ་ཤིས" + "ཀྱིས").
pub fn split_affixed(tokens: &mut Vec<Token>) {
    let mut i = 0;
    while i < tokens.len() {
        // Check if this token has affixation info and should be split
        if let Some(ref affixation) = tokens[i].affixation {
            // Check that there's no sense that explicitly says "affixed: false"
            let should_split = !tokens[i].senses.iter().any(|s| !s.affixed);

            if should_split && tokens[i].syls.len() > 1 {
                let affix_len = affixation.len;
                
                // Calculate split point
                if let Some(last_syl) = tokens[i].syls.last() {
                    if last_syl.chars().count() >= affix_len {
                        // Split the token
                        let (host, particle) = split_token_at_affix(&tokens[i], affix_len);
                        
                        // Replace original with host and insert particle
                        tokens[i] = host;
                        tokens.insert(i + 1, particle);
                        
                        i += 1; // Skip the newly inserted particle
                    }
                }
            }
        }
        i += 1;
    }
}

/// Split a token at the affix boundary
fn split_token_at_affix(token: &Token, affix_len: usize) -> (Token, Token) {
    let syls = &token.syls;
    
    // Find the split point in the text
    // The affix is at the end of the last syllable
    let last_syl = syls.last().unwrap();
    let last_syl_chars: Vec<char> = last_syl.chars().collect();
    let split_char_idx = last_syl_chars.len() - affix_len;
    
    // Calculate byte position for split
    let host_syl: String = last_syl_chars[..split_char_idx].iter().collect();
    let particle_syl: String = last_syl_chars[split_char_idx..].iter().collect();
    
    // Create host token (all but the affix)
    let mut host_syls: Vec<String> = syls[..syls.len() - 1].to_vec();
    if !host_syl.is_empty() {
        host_syls.push(host_syl);
    }
    
    let host_text = host_syls.join(&TSEK.to_string());
    let host_len = host_text.len();
    
    let mut host = Token::with_text(
        host_text,
        token.start,
        host_len,
        ChunkType::Text,
    );
    host.syls = host_syls;
    host.pos = token.pos.clone();
    host.lemma = token.lemma.clone();
    host.freq = token.freq;
    host.is_affix_host = true;
    host.senses = token.senses.clone();
    
    // Create particle token
    let particle_text = format!("{}{}", particle_syl, TSEK);
    let particle_start = token.start + host_len;
    let particle_len = particle_text.len();
    
    let mut particle = Token::with_text(
        particle_text,
        particle_start,
        particle_len,
        ChunkType::Text,
    );
    particle.syls = vec![particle_syl];
    particle.pos = Some("PART".to_string());
    particle.is_affix = true;
    
    (host, particle)
}

/// Merge dagdra particles (པ་/པོ་/བ་/བོ་) with the preceding word.
///
/// In Tibetan, these particles are often written separately but should be
/// considered part of the preceding word for many NLP tasks.
pub fn merge_dagdra(tokens: &mut Vec<Token>) {
    if tokens.len() <= 1 {
        return;
    }

    let mut i = 0;
    while i < tokens.len() - 1 {
        let is_text_pair = tokens[i].chunk_type == ChunkType::Text
            && tokens[i + 1].chunk_type == ChunkType::Text;
        
        if is_text_pair && is_dagdra(&tokens[i + 1].text_cleaned()) {
            // Merge the dagdra with the previous token
            let merged = merge_two_tokens(&tokens[i], &tokens[i + 1]);
            tokens[i] = merged;
            tokens.remove(i + 1);
            // Don't increment i - check if the new merged token can merge again
        } else {
            i += 1;
        }
    }
}

/// Merge two tokens into one
fn merge_two_tokens(first: &Token, second: &Token) -> Token {
    let merged_text = format!("{}{}", first.text, second.text);
    let merged_len = first.len + second.len;
    
    let mut merged = Token::with_text(
        merged_text,
        first.start,
        merged_len,
        ChunkType::Text,
    );
    
    // Combine syllables
    let mut merged_syls = first.syls.clone();
    merged_syls.extend(second.syls.clone());
    merged.syls = merged_syls;
    
    // Keep first token's linguistic info, mark as merged
    merged.pos = first.pos.clone();
    merged.freq = first.freq;
    merged.has_merged_dagdra = true;
    
    // Generate lemma from cleaned text
    merged.lemma = Some(merged.text_cleaned());
    
    merged
}

/// Generate default lemmas for tokens that don't have one.
///
/// The default lemma is the cleaned text (syllables joined with tsek).
pub fn generate_default_lemmas(tokens: &mut [Token]) {
    for token in tokens.iter_mut() {
        if token.lemma.is_none() && !token.syls.is_empty() {
            token.lemma = Some(token.text_cleaned());
        }
    }
}

/// Choose the default/best sense for each token.
///
/// When a token has multiple possible senses, select the most likely one
/// based on frequency or other heuristics.
pub fn choose_default_senses(tokens: &mut [Token]) {
    for token in tokens.iter_mut() {
        if token.senses.len() > 1 {
            // Sort by frequency (highest first) and take the first
            token.senses.sort_by(|a, b| {
                let freq_a = a.freq.unwrap_or(0);
                let freq_b = b.freq.unwrap_or(0);
                freq_b.cmp(&freq_a)
            });
            
            // Use the best sense's POS if token doesn't have one
            if token.pos.is_none() {
                if let Some(best_sense) = token.senses.first() {
                    token.pos = best_sense.pos.clone();
                }
            }
        }
    }
}

/// Apply all post-processing steps to a token list.
///
/// This is the main entry point for token modification, applying:
/// 1. Split affixed particles
/// 2. Merge dagdra particles
/// 3. Generate default lemmas
/// 4. Choose default senses
pub fn apply_all_modifiers(tokens: &mut Vec<Token>, split_affixes: bool) {
    if split_affixes {
        split_affixed(tokens);
    }
    merge_dagdra(tokens);
    generate_default_lemmas(tokens);
    choose_default_senses(tokens);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_dagdra() {
        let mut tokens = vec![
            Token::with_text("བཀྲ་ཤིས་".to_string(), 0, 18, ChunkType::Text),
            Token::with_text("པ་".to_string(), 18, 6, ChunkType::Text),
        ];
        tokens[0].syls = vec!["བཀྲ".to_string(), "ཤིས".to_string()];
        tokens[1].syls = vec!["པ".to_string()];

        merge_dagdra(&mut tokens);

        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].has_merged_dagdra);
        assert_eq!(tokens[0].syls.len(), 3);
    }

    #[test]
    fn test_generate_default_lemmas() {
        let mut tokens = vec![
            Token::with_text("བཀྲ་ཤིས་".to_string(), 0, 18, ChunkType::Text),
        ];
        tokens[0].syls = vec!["བཀྲ".to_string(), "ཤིས".to_string()];

        generate_default_lemmas(&mut tokens);

        assert!(tokens[0].lemma.is_some());
    }
}

