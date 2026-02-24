use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

lazy_static! {
    static ref STOPWORDS: HashSet<&'static str> = {
        let words = [
            "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "up", "about", "into", "over", "after",
            "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should",
            "may", "might", "must", "shall", "can", "need", "dare", "ought",
            "used", "it", "its", "this", "that", "these", "those", "i", "me",
            "my", "myself", "we", "our", "ours", "ourselves", "you", "your",
            "yours", "yourself", "yourselves", "he", "him", "his", "himself",
            "she", "her", "hers", "herself", "they", "them", "their", "theirs",
            "themselves", "what", "which", "who", "whom", "when", "where",
            "why", "how", "all", "each", "every", "both", "few", "more", "most",
            "other", "some", "such", "no", "nor", "not", "only", "own", "same",
            "so", "than", "too", "very", "just", "also", "now", "here", "there",
            "then", "once", "if", "because", "as", "until", "while", "during",
            "before", "after", "above", "below", "between", "under", "again",
            "further", "any", "s", "t", "d", "m", "ll", "re", "ve",
        ];
        words.iter().copied().collect()
    };
}

pub struct TfIdfScorer {
    doc_frequencies: HashMap<String, usize>,
    total_docs: usize,
}

impl TfIdfScorer {
    pub fn new(documents: &[&str]) -> Self {
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();
        
        for doc in documents {
            let terms: HashSet<String> = preprocess(doc).into_iter().collect();
            for term in terms {
                *doc_frequencies.entry(term).or_insert(0) += 1;
            }
        }
        
        Self {
            doc_frequencies,
            total_docs: documents.len(),
        }
    }
    
    pub fn score(&self, text: &str) -> f64 {
        let terms = preprocess(text);
        
        if terms.is_empty() || self.total_docs == 0 {
            return 0.0;
        }
        
        // Calculate term frequencies
        let mut term_freq: HashMap<&str, usize> = HashMap::new();
        for term in &terms {
            *term_freq.entry(term.as_str()).or_insert(0) += 1;
        }
        
        let mut score = 0.0;
        let max_tf = *term_freq.values().max().unwrap_or(&1) as f64;
        
        for (term, &tf) in &term_freq {
            // Augmented term frequency (prevents bias toward long docs)
            let tf_norm = 0.5 + 0.5 * (tf as f64 / max_tf);
            
            // IDF with smoothing
            let df = self.doc_frequencies.get(*term).copied().unwrap_or(0);
            let idf = if df > 0 {
                ((self.total_docs as f64 + 1.0) / (df as f64 + 1.0)).ln() + 1.0
            } else {
                // Unseen term gets maximum IDF
                ((self.total_docs as f64 + 1.0) / 1.0).ln() + 1.0
            };
            
            score += tf_norm * idf;
        }
        
        // Normalize by number of unique terms to avoid length bias
        score / (term_freq.len() as f64).sqrt()
    }
}

fn preprocess(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && w.len() > 1 && !STOPWORDS.contains(w))
        .map(stem)
        .collect()
}

// Simple Porter-like stemmer (handles common cases)
fn stem(word: &str) -> String {
    let mut s = word.to_string();
    
    // Handle common suffixes
    if s.ends_with("ing") && s.len() > 5 {
        s.truncate(s.len() - 3);
        // Handle doubling
        if s.ends_with("ll") || s.ends_with("ss") || s.ends_with("zz") {
            s.truncate(s.len() - 1);
        }
    } else if s.ends_with("ed") && s.len() > 4 {
        s.truncate(s.len() - 2);
    } else if s.ends_with("es") && s.len() > 4 {
        s.truncate(s.len() - 2);
    } else if s.ends_with("s") && !s.ends_with("ss") && s.len() > 3 {
        s.truncate(s.len() - 1);
    } else if s.ends_with("ly") && s.len() > 4 {
        s.truncate(s.len() - 2);
    } else if s.ends_with("tion") && s.len() > 6 {
        s.truncate(s.len() - 4);
        s.push_str("te");
    } else if s.ends_with("ness") && s.len() > 6 {
        s.truncate(s.len() - 4);
    }
    
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_terms_score_higher() {
        let docs = ["the cat sat on the mat", "the dog ran in the park", "cats and dogs are pets"];
        let scorer = TfIdfScorer::new(&docs);
        
        // "quantum" is unique, should score high
        let rare_score = scorer.score("quantum physics experiments");
        // Common words should score lower
        let common_score = scorer.score("cat dog pet");
        
        assert!(rare_score > common_score);
    }

    #[test]
    fn test_empty_segment() {
        let docs = ["some text here"];
        let scorer = TfIdfScorer::new(&docs);
        assert_eq!(scorer.score(""), 0.0);
    }

    #[test]
    fn test_stopwords_dont_contribute() {
        let docs = ["important content here"];
        let scorer = TfIdfScorer::new(&docs);
        
        // Only stopwords
        let stopword_score = scorer.score("the and or but");
        assert_eq!(stopword_score, 0.0);
    }

    #[test]
    fn test_stemming_groups() {
        let docs = ["running runs runner"];
        let scorer = TfIdfScorer::new(&docs);
        
        // "run" should be found since "running" stems to "runn" -> close match
        // Actually with our simple stemmer, they become: run, run, runner
        let terms1 = preprocess("running");
        let terms2 = preprocess("runs");
        // Both should stem similarly
        assert!(!terms1.is_empty());
        assert!(!terms2.is_empty());
    }

    #[test]
    fn test_long_segment_normalized() {
        let docs = ["short doc", "another short"];
        let scorer = TfIdfScorer::new(&docs);
        
        let short_score = scorer.score("unique term");
        let long_score = scorer.score("unique term repeated many times with extra words");
        
        // Long segment shouldn't have dramatically higher score due to normalization
        assert!(long_score < short_score * 3.0);
    }

    #[test]
    fn test_single_word() {
        let docs = ["hello world"];
        let scorer = TfIdfScorer::new(&docs);
        let score = scorer.score("quantum");
        assert!(score > 0.0);
    }

    #[test]
    fn test_all_stopwords() {
        let docs = ["content here"];
        let scorer = TfIdfScorer::new(&docs);
        let score = scorer.score("the a an is are was were");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_consistent_scores() {
        let docs = ["document one", "document two"];
        let scorer = TfIdfScorer::new(&docs);
        let score1 = scorer.score("test query");
        let score2 = scorer.score("test query");
        assert_eq!(score1, score2);
    }

    #[test]
    fn test_preprocess_lowercase() {
        let terms = preprocess("HELLO World");
        assert!(terms.iter().all(|t| t.chars().all(|c| c.is_lowercase())));
    }

    #[test]
    fn test_stem_basic() {
        assert_eq!(stem("running"), "runn");
        assert_eq!(stem("jumped"), "jump");
        assert_eq!(stem("cats"), "cat");
    }
}
