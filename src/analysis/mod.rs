pub mod minhash;
pub mod tfidf;
pub mod boilerplate;

pub use minhash::{MinHash, MinHashSignature};
pub use tfidf::TfIdfScorer;
pub use boilerplate::BoilerplateDetector;

use crate::segmenter::Segment;
use crate::tokenizer::Tokenizer;
use crate::config::Config;
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub segments: Vec<AnalyzedSegment>,
    pub total_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct AnalyzedSegment {
    #[allow(dead_code)]
    pub index: usize,
    pub segment: Segment,
    pub minhash: MinHashSignature,
    pub tfidf_score: f64,
    #[allow(dead_code)]
    pub boilerplate_score: f64,
    pub importance_score: f64,
    pub is_redundant: bool,
    pub is_boilerplate: bool,
    /// Whether this segment must be preserved (e.g., code blocks when --preserve-code)
    pub is_preserved: bool,
}

pub fn analyze(
    mut segments: Vec<Segment>,
    tokenizer: &dyn Tokenizer,
    config: &Config,
) -> AnalysisResult {
    if segments.is_empty() {
        return AnalysisResult {
            segments: vec![],
            total_tokens: 0,
        };
    }
    
    // Count tokens
    tokenizer.count_segments(&mut segments);
    let total_tokens: usize = segments.iter().map(|s| s.tokens).sum();
    
    // Build TF-IDF scorer
    let texts: Vec<&str> = segments.iter().map(|s| s.text.as_str()).collect();
    let tfidf = TfIdfScorer::new(&texts);
    
    // Boilerplate detector with config patterns
    let detector = BoilerplateDetector::new()
        .with_additional_patterns(&config.boilerplate_patterns);
    
    // Analyze each segment in parallel
    let analyzed: Vec<AnalyzedSegment> = segments
        .into_par_iter()
        .enumerate()
        .map(|(index, segment)| {
            let minhash_sig = MinHash::compute_signature(&segment.text);
            let tfidf_score = tfidf.score(&segment.text);
            let boilerplate_score = detector.detect(&segment.text);
            
            // Initial importance is just TF-IDF, refined later
            let importance_score = tfidf_score;
            
            // Mark as boilerplate if above threshold
            let is_boilerplate = boilerplate_score >= config.thresholds.boilerplate_confidence;
            
            // Code blocks are preserved when is_code_block is set
            let is_preserved = segment.is_code_block;
            
            AnalyzedSegment {
                index,
                segment,
                minhash: minhash_sig,
                tfidf_score,
                boilerplate_score,
                importance_score,
                is_redundant: false,  // Set later by redundancy detection
                is_boilerplate,
                is_preserved,
            }
        })
        .collect();
    
    // Detect redundancy using MinHash clustering
    let signatures: Vec<MinHashSignature> = analyzed.iter().map(|a| a.minhash.clone()).collect();
    let scores: Vec<f64> = analyzed.iter().map(|a| a.importance_score).collect();
    let clusters = MinHash::find_clusters(&signatures, &scores, config.thresholds.redundancy_similarity);
    
    // Mark redundant segments (not the cluster representative)
    let mut final_segments: Vec<AnalyzedSegment> = analyzed;
    for (i, seg) in final_segments.iter_mut().enumerate() {
        if clusters[i] != i {
            seg.is_redundant = true;
        }
    }
    
    // Apply position decay to importance scores
    let len = final_segments.len() as f64;
    for (i, seg) in final_segments.iter_mut().enumerate() {
        // Segments near start and end get boosted
        let position = i as f64 / len;
        let position_weight = if !(0.1..=0.9).contains(&position) {
            1.0  // First and last 10% get full weight
        } else {
            config.thresholds.position_decay
        };
        seg.importance_score *= position_weight;
    }
    
    AnalysisResult {
        segments: final_segments,
        total_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::ApproximateTokenizer;
    use crate::config;
    use crate::cli::Args;
    use clap::Parser;

    fn test_config() -> Config {
        let args = Args::parse_from(["test", "--budget", "1000"]);
        config::load(None).and_then(|c| config::merge(c, &args)).unwrap()
    }

    #[test]
    fn test_analyze_empty() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let result = analyze(vec![], &tokenizer, &config);
        assert!(result.segments.is_empty());
        assert_eq!(result.total_tokens, 0);
    }

    #[test]
    fn test_analyze_single() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let segments = vec![
            Segment { text: "Hello world test".to_string(), start: 0, end: 16, tokens: 0, is_code_block: false, heading_level: 0 },
        ];
        let result = analyze(segments, &tokenizer, &config);
        assert_eq!(result.segments.len(), 1);
        assert!(result.total_tokens > 0);
        assert!(!result.segments[0].is_redundant);
    }

    #[test]
    fn test_analyze_redundancy() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let segments = vec![
            Segment { text: "The quick brown fox jumps over".to_string(), start: 0, end: 30, tokens: 0, is_code_block: false, heading_level: 0 },
            Segment { text: "The quick brown fox jumps over".to_string(), start: 32, end: 62, tokens: 0, is_code_block: false, heading_level: 0 },
            Segment { text: "Completely different unique content".to_string(), start: 64, end: 100, tokens: 0, is_code_block: false, heading_level: 0 },
        ];
        let result = analyze(segments, &tokenizer, &config);
        assert_eq!(result.segments.len(), 3);
        // One of the duplicates should be marked redundant
        let redundant_count = result.segments.iter().filter(|s| s.is_redundant).count();
        assert!(redundant_count >= 1);
    }

    #[test]
    fn test_analyze_boilerplate() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let segments = vec![
            Segment { text: "MIT License Copyright".to_string(), start: 0, end: 20, tokens: 0, is_code_block: false, heading_level: 0 },
            Segment { text: "Normal content here".to_string(), start: 22, end: 40, tokens: 0, is_code_block: false, heading_level: 0 },
        ];
        let result = analyze(segments, &tokenizer, &config);
        assert!(result.segments[0].is_boilerplate);
        assert!(!result.segments[1].is_boilerplate);
    }

    #[test]
    fn test_analyze_importance_scores() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let segments = vec![
            Segment { text: "Common words here today".to_string(), start: 0, end: 23, tokens: 0, is_code_block: false, heading_level: 0 },
            Segment { text: "Quantum physics experiments".to_string(), start: 25, end: 52, tokens: 0, is_code_block: false, heading_level: 0 },
        ];
        let result = analyze(segments, &tokenizer, &config);
        // All segments should have importance scores
        for seg in &result.segments {
            assert!(seg.importance_score > 0.0 || seg.is_boilerplate);
        }
    }

    #[test]
    fn test_analyze_tokens_counted() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let segments = vec![
            Segment { text: "one two three four".to_string(), start: 0, end: 18, tokens: 0, is_code_block: false, heading_level: 0 },
        ];
        let result = analyze(segments, &tokenizer, &config);
        assert!(result.segments[0].segment.tokens > 0);
        assert_eq!(result.total_tokens, result.segments[0].segment.tokens);
    }

    #[test]
    fn test_analyze_code_blocks_preserved() {
        let tokenizer = ApproximateTokenizer;
        let config = test_config();
        let segments = vec![
            Segment { text: "Normal text here".to_string(), start: 0, end: 16, tokens: 0, is_code_block: false, heading_level: 0 },
            Segment { text: "```rust\nfn main() {}\n```".to_string(), start: 18, end: 42, tokens: 0, is_code_block: true, heading_level: 0 },
        ];
        let result = analyze(segments, &tokenizer, &config);
        assert!(!result.segments[0].is_preserved);
        assert!(result.segments[1].is_preserved);
    }
}
