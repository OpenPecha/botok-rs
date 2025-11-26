//! Trie data structure for dictionary-based tokenization.
//!
//! The Trie stores words (as sequences of syllables) and allows for efficient
//! longest-match lookups during tokenization.

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
    /// Whether 'aa' (འ) is added
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
}

/// Builder for loading a Trie from TSV files
pub struct TrieBuilder {
    trie: Trie,
}

impl TrieBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        TrieBuilder { trie: Trie::new() }
    }

    /// Load words from a TSV string (format: form\tpos\tlemma\tsense\tfreq)
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
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            let lemma = parts.get(2).and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            let _sense = parts.get(3); // Currently unused
            let freq = parts
                .get(4)
                .and_then(|s| s.trim().parse::<u32>().ok());

            let data = WordData {
                pos: pos.clone(),
                lemma,
                freq,
                ..Default::default()
            };

            // Add the sense
            let sense = Sense {
                pos,
                freq,
                ..Default::default()
            };

            self.trie.add_word(form, Some(data));
            
            // Split into syllables and add sense
            let syls: Vec<&str> = form
                .split('་')
                .filter(|s| !s.is_empty())
                .collect();
            if !syls.is_empty() {
                self.trie.add_data(&syls, sense);
            }
        }
    }

    /// Build and return the Trie
    pub fn build(self) -> Trie {
        self.trie
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

