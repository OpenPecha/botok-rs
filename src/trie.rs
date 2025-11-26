//! Trie data structure for dictionary-based tokenization.
//!
//! The Trie stores words (as sequences of syllables) and allows for efficient
//! longest-match lookups during tokenization.
//!
//! ## Auto-Inflection
//! 
//! When loading words from TSV files, the `TrieBuilder` can automatically generate
//! all affixed forms of each word. This is essential for Tibetan NLP since Tibetan
//! has productive affixation (particles like འི, ས, ར, etc. attach to words).

use crate::syllable::{AffixData, SylComponents};
use crate::token::Sense;
use std::collections::HashMap;

/// Data associated with a word in the Trie
#[derive(Debug, Clone, Default)]
pub struct WordData {
    /// Part-of-speech tag
    pub pos: Option<String>,
    /// Lemma (base form)
    pub lemma: Option<String>,
    /// Frequency
    pub freq: Option<u32>,
    /// Whether this is a Sanskrit word
    pub skrt: bool,
    /// Affixation information
    pub affixation: Option<AffixInfo>,
    /// Multiple senses/meanings
    pub senses: Vec<Sense>,
}

/// Information about how a word can be affixed
#[derive(Debug, Clone)]
pub struct AffixInfo {
    /// Length of the affix in characters
    pub len: usize,
    /// Type of affix (e.g., "la", "gis", "gi", etc.)
    pub affix_type: String,
    /// Whether 'aa' (འ) was removed before adding the affix
    pub aa: bool,
}

/// A node in the Trie
#[derive(Debug, Clone, Default)]
pub struct TrieNode {
    /// Children nodes, keyed by syllable
    pub children: HashMap<String, TrieNode>,
    /// Whether this node marks the end of a valid word
    pub is_leaf: bool,
    /// Data associated with this word (if is_leaf is true)
    pub data: Option<WordData>,
}

impl TrieNode {
    /// Create a new empty node
    pub fn new() -> Self {
        TrieNode::default()
    }

    /// Check if this node has any children
    pub fn can_walk(&self) -> bool {
        !self.children.is_empty()
    }

    /// Check if this node is a valid word ending
    pub fn is_match(&self) -> bool {
        self.is_leaf
    }
}

/// A Trie for storing and looking up Tibetan words
#[derive(Debug, Default, Clone)]
pub struct Trie {
    /// The root node
    root: TrieNode,
    /// Number of words in the trie
    word_count: usize,
}

impl Trie {
    /// Create a new empty Trie
    pub fn new() -> Self {
        Trie::default()
    }

    /// Get the number of words in the trie
    pub fn len(&self) -> usize {
        self.word_count
    }

    /// Check if the trie is empty
    pub fn is_empty(&self) -> bool {
        self.word_count == 0
    }

    /// Add a word (as a slice of syllables) to the trie
    pub fn add(&mut self, syls: &[&str], data: Option<WordData>) {
        let mut current = &mut self.root;

        for syl in syls {
            current = current
                .children
                .entry(syl.to_string())
                .or_insert_with(TrieNode::new);
        }

        if !current.is_leaf {
            self.word_count += 1;
        }
        current.is_leaf = true;

        if let Some(d) = data {
            current.data = Some(d);
        }
    }

    /// Add a word from a string (will be split into syllables by tsek)
    pub fn add_word(&mut self, word: &str, data: Option<WordData>) {
        let syls: Vec<&str> = word
            .split('་')
            .filter(|s| !s.is_empty())
            .collect();
        
        if !syls.is_empty() {
            self.add(&syls, data);
        }
    }

    /// Add a word and return a mutable reference to the node for further modification.
    /// This avoids double traversal when you need to add data after adding the word.
    pub fn add_word_and_get_node(&mut self, word: &str, data: Option<WordData>) -> Option<&mut TrieNode> {
        let syls: Vec<&str> = word
            .split('་')
            .filter(|s| !s.is_empty())
            .collect();
        
        if syls.is_empty() {
            return None;
        }

        let mut current = &mut self.root;

        for syl in &syls {
            current = current
                .children
                .entry(syl.to_string())
                .or_insert_with(TrieNode::new);
        }

        if !current.is_leaf {
            self.word_count += 1;
        }
        current.is_leaf = true;

        if let Some(d) = data {
            current.data = Some(d);
        }

        Some(current)
    }

    /// Add a word with sense data in a single traversal (optimized for TSV loading)
    pub fn add_word_with_sense(&mut self, word: &str, data: WordData, sense: Sense) {
        let syls: Vec<&str> = word
            .split('་')
            .filter(|s| !s.is_empty())
            .collect();
        
        if syls.is_empty() {
            return;
        }

        let mut current = &mut self.root;

        for syl in &syls {
            current = current
                .children
                .entry(syl.to_string())
                .or_insert_with(TrieNode::new);
        }

        if !current.is_leaf {
            self.word_count += 1;
        }
        current.is_leaf = true;

        // Merge data and sense in one operation
        if let Some(ref mut existing_data) = current.data {
            // Update existing data if needed
            if existing_data.pos.is_none() && data.pos.is_some() {
                existing_data.pos = data.pos;
            }
            if existing_data.lemma.is_none() && data.lemma.is_some() {
                existing_data.lemma = data.lemma;
            }
            if existing_data.freq.is_none() && data.freq.is_some() {
                existing_data.freq = data.freq;
            }
            existing_data.senses.push(sense);
        } else {
            let mut new_data = data;
            new_data.senses.push(sense);
            current.data = Some(new_data);
        }
    }

    /// Walk the trie by one syllable, returning the next node if it exists
    pub fn walk<'a>(&'a self, syl: &str, current: Option<&'a TrieNode>) -> Option<&'a TrieNode> {
        let node = current.unwrap_or(&self.root);
        node.children.get(syl)
    }

    /// Check if a word exists in the trie
    pub fn has_word(&self, syls: &[&str]) -> bool {
        let mut current = &self.root;

        for syl in syls {
            match current.children.get(*syl) {
                Some(node) => current = node,
                None => return false,
            }
        }

        current.is_leaf
    }

    /// Get the data for a word if it exists
    pub fn get_word_data(&self, syls: &[&str]) -> Option<&WordData> {
        let mut current = &self.root;

        for syl in syls {
            match current.children.get(*syl) {
                Some(node) => current = node,
                None => return None,
            }
        }

        if current.is_leaf {
            current.data.as_ref()
        } else {
            None
        }
    }

    /// Add data to an existing word
    pub fn add_data(&mut self, syls: &[&str], sense: Sense) -> bool {
        let mut current = &mut self.root;

        for syl in syls {
            match current.children.get_mut(*syl) {
                Some(node) => current = node,
                None => return false,
            }
        }

        if !current.is_leaf {
            return false;
        }

        if current.data.is_none() {
            current.data = Some(WordData::default());
        }

        if let Some(ref mut data) = current.data {
            data.senses.push(sense);
        }

        true
    }

    /// Deactivate a word (make it not findable)
    pub fn deactivate(&mut self, syls: &[&str]) -> bool {
        let mut current = &mut self.root;

        for syl in syls {
            match current.children.get_mut(*syl) {
                Some(node) => current = node,
                None => return false,
            }
        }

        if current.is_leaf {
            current.is_leaf = false;
            self.word_count -= 1;
            true
        } else {
            false
        }
    }

    /// Get a reference to the root node (for external traversal)
    pub fn root(&self) -> &TrieNode {
        &self.root
    }

    /// Merge another trie into this one
    pub fn merge(&mut self, other: &Trie) {
        let added = Self::merge_nodes_recursive(&mut self.root, &other.root);
        self.word_count += added;
    }

    fn merge_nodes_recursive(target: &mut TrieNode, source: &TrieNode) -> usize {
        let mut added = 0;
        
        for (syl, source_child) in &source.children {
            let target_child = target.children
                .entry(syl.clone())
                .or_insert_with(TrieNode::new);
            
            if source_child.is_leaf && !target_child.is_leaf {
                target_child.is_leaf = true;
                added += 1;
            }
            
            if source_child.is_leaf && source_child.data.is_some() {
                target_child.data = source_child.data.clone();
            }
            
            // Recursively merge children
            added += Self::merge_nodes_recursive(target_child, source_child);
        }
        
        added
    }
}

/// Builder for loading a Trie from TSV files
/// 
/// Supports auto-inflection: when `inflect` is enabled, all affixed forms
/// of each word are automatically generated and added to the trie.
pub struct TrieBuilder {
    trie: Trie,
    /// Syllable components for inflection
    syl_components: SylComponents,
    /// Whether to auto-generate inflected forms
    inflect: bool,
    /// Cache for inflected forms to avoid recomputation
    inflection_cache: HashMap<String, Vec<(Vec<String>, Option<AffixData>)>>,
}

impl TrieBuilder {
    /// Create a new builder with inflection disabled
    pub fn new() -> Self {
        TrieBuilder { 
            trie: Trie::new(),
            syl_components: SylComponents::new(),
            inflect: false,
            inflection_cache: HashMap::new(),
        }
    }

    /// Create a new builder with inflection enabled
    pub fn with_inflection() -> Self {
        TrieBuilder {
            trie: Trie::new(),
            syl_components: SylComponents::new(),
            inflect: true,
            inflection_cache: HashMap::new(),
        }
    }

    /// Enable or disable auto-inflection
    pub fn set_inflection(&mut self, enable: bool) -> &mut Self {
        self.inflect = enable;
        self
    }

    /// Get all inflected forms of a word
    /// 
    /// Returns a list of (syllables, affix_data) tuples.
    /// The first element is always the base form with None affix_data.
    fn get_inflected(&mut self, word: &str) -> Vec<(Vec<String>, Option<AffixData>)> {
        // Check cache first
        if let Some(cached) = self.inflection_cache.get(word) {
            return cached.clone();
        }

        let syls: Vec<String> = word
            .split('་')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if syls.is_empty() {
            return vec![];
        }

        // Start with the base form
        let mut inflected = vec![(syls.clone(), None)];

        // Get affixed forms of the last syllable
        if let Some(last_syl) = syls.last() {
            if let Some(affixed) = self.syl_components.get_all_affixed(last_syl) {
                for (affixed_syl, affix_data) in affixed {
                    let mut inflected_word = syls[..syls.len() - 1].to_vec();
                    inflected_word.push(affixed_syl);
                    inflected.push((inflected_word, Some(affix_data)));
                }
            }
        }

        // Cache the result
        self.inflection_cache.insert(word.to_string(), inflected.clone());
        inflected
    }

    /// Load words from a TSV string (format: form\tpos\tlemma\tsense\tfreq)
    /// 
    /// If inflection is enabled, automatically generates all affixed forms.
    /// Uses single-traversal optimization to avoid double trie walks.
    pub fn load_tsv(&mut self, tsv_content: &str) {
        for line in tsv_content.lines() {
            // Skip comments and empty lines
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.is_empty() {
                continue;
            }

            let form = parts[0];
            let pos = parts.get(1).and_then(|s| {
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            let lemma = parts.get(2).and_then(|s| {
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            let sense_text = parts.get(3).and_then(|s| {
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            let freq = parts.get(4).and_then(|s| s.trim().parse::<u32>().ok());

            if self.inflect {
                // Get all inflected forms
                let inflected = self.get_inflected(form);
                
                for (syls, affix_data) in inflected {
                    let is_affixed = affix_data.is_some();
                    
                    // Build word data
                    let data = WordData {
                        pos: pos.clone(),
                        lemma: lemma.clone(),
                        freq,
                        affixation: affix_data.map(|a| AffixInfo {
                            len: a.len,
                            affix_type: a.affix_type,
                            aa: a.aa,
                        }),
                        ..Default::default()
                    };

                    // Build sense
                    let sense = Sense {
                        pos: pos.clone(),
                        freq,
                        sense: sense_text.clone(),
                        affixed: is_affixed,
                        ..Default::default()
                    };

                    // Add word with sense in single traversal
                    let word = syls.join("་");
                    self.trie.add_word_with_sense(&word, data, sense);
                }
            } else {
                // Non-inflected mode: just add the word as-is
                let data = WordData {
                    pos: pos.clone(),
                    lemma: lemma.clone(),
                    freq,
                    ..Default::default()
                };

                let sense = Sense {
                    pos: pos.clone(),
                    freq,
                    sense: sense_text.clone(),
                    ..Default::default()
                };

                // Single traversal: add word with sense together
                self.trie.add_word_with_sense(form, data, sense);
            }
        }
    }

    /// Add a word with all its inflected forms (for dynamic word addition)
    pub fn add_inflected_word(&mut self, word: &str, data: Option<WordData>) {
        if self.inflect {
            let inflected = self.get_inflected(word);
            
            for (syls, affix_data) in inflected {
                let mut word_data = data.clone().unwrap_or_default();
                word_data.affixation = affix_data.map(|a| AffixInfo {
                    len: a.len,
                    affix_type: a.affix_type,
                    aa: a.aa,
                });
                
                let word_str = syls.join("་");
                self.trie.add_word(&word_str, Some(word_data));
            }
        } else {
            self.trie.add_word(word, data);
        }
    }

    /// Deactivate a word and all its inflected forms
    pub fn deactivate_inflected_word(&mut self, word: &str) {
        if self.inflect {
            let inflected = self.get_inflected(word);
            
            for (syls, _) in inflected {
                let syls_ref: Vec<&str> = syls.iter().map(|s| s.as_str()).collect();
                self.trie.deactivate(&syls_ref);
            }
        } else {
            let syls: Vec<&str> = word.split('་').filter(|s| !s.is_empty()).collect();
            self.trie.deactivate(&syls);
        }
    }

    /// Build and return the Trie
    pub fn build(self) -> Trie {
        self.trie
    }
    
    /// Get a reference to the underlying trie (for inspection)
    pub fn trie(&self) -> &Trie {
        &self.trie
    }
    
    /// Get a mutable reference to the underlying trie (for advanced usage)
    pub fn trie_mut(&mut self) -> &mut Trie {
        &mut self.trie
    }
}

impl Default for TrieBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_add_and_lookup() {
        let mut trie = Trie::new();

        trie.add(&["བཀྲ", "ཤིས"], None);
        trie.add(&["བདེ", "ལེགས"], None);

        assert!(trie.has_word(&["བཀྲ", "ཤིས"]));
        assert!(trie.has_word(&["བདེ", "ལེགས"]));
        assert!(!trie.has_word(&["བཀྲ"])); // Partial word
        assert!(!trie.has_word(&["བཀྲ", "ཤིས", "བདེ"])); // Non-existent
    }

    #[test]
    fn test_trie_walk() {
        let mut trie = Trie::new();
        trie.add(&["བཀྲ", "ཤིས"], None);

        let node1 = trie.walk("བཀྲ", None);
        assert!(node1.is_some());
        assert!(!node1.unwrap().is_match()); // Not a complete word yet

        let node2 = trie.walk("ཤིས", node1);
        assert!(node2.is_some());
        assert!(node2.unwrap().is_match()); // Complete word
    }

    #[test]
    fn test_trie_with_data() {
        let mut trie = Trie::new();

        let data = WordData {
            pos: Some("NOUN".to_string()),
            freq: Some(1000),
            ..Default::default()
        };

        trie.add(&["བཀྲ", "ཤིས"], Some(data));

        let retrieved = trie.get_word_data(&["བཀྲ", "ཤིས"]);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().pos, Some("NOUN".to_string()));
        assert_eq!(retrieved.unwrap().freq, Some(1000));
    }

    #[test]
    fn test_trie_builder() {
        let tsv = "བཀྲ་ཤིས\tNOUN\t\t\t1000\nབདེ་ལེགས\tNOUN\t\t\t500";

        let mut builder = TrieBuilder::new();
        builder.load_tsv(tsv);
        let trie = builder.build();

        assert_eq!(trie.len(), 2);
        assert!(trie.has_word(&["བཀྲ", "ཤིས"]));
        assert!(trie.has_word(&["བདེ", "ལེགས"]));
    }

    #[test]
    fn test_add_word_string() {
        let mut trie = Trie::new();
        trie.add_word("བཀྲ་ཤིས་བདེ་ལེགས", None);

        assert!(trie.has_word(&["བཀྲ", "ཤིས", "བདེ", "ལེགས"]));
    }
}

