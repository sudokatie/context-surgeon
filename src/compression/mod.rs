pub mod budget;
pub mod redundancy;
pub mod extractive;

pub use budget::BudgetAllocator;
pub use redundancy::RedundancyRemover;
pub use extractive::ExtractiveCompressor;

use crate::analysis::AnalysisResult;
use crate::config::Config;
use crate::segmenter;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("budget too small: need at least {minimum} tokens, got {actual}")]
    BudgetTooSmall { minimum: usize, actual: usize },
    #[error("no segments to compress")]
    NoSegments,
    #[error("budget error: {0}")]
    Budget(#[from] budget::BudgetError),
}

#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub output: String,
    pub stats: CompressionStats,
}

#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub redundancy_removed: usize,
    pub boilerplate_removed: usize,
    pub extractive_removed: usize,
    pub segments_kept: usize,
    pub segments_total: usize,
}

impl CompressionStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.original_tokens == 0 {
            return 1.0;
        }
        self.compressed_tokens as f64 / self.original_tokens as f64
    }
    
    pub fn tokens_saved(&self) -> usize {
        self.original_tokens.saturating_sub(self.compressed_tokens)
    }
}

pub fn compress(
    mut analysis: AnalysisResult,
    budget: usize,
    preserve_head: usize,
    preserve_tail: usize,
    config: &Config,
) -> Result<CompressionResult, CompressionError> {
    let original_tokens = analysis.total_tokens;
    let segments_total = analysis.segments.len();
    
    if analysis.segments.is_empty() {
        return Err(CompressionError::NoSegments);
    }
    
    // Check budget feasibility
    if original_tokens <= budget {
        // No compression needed
        let indices: Vec<usize> = (0..analysis.segments.len()).collect();
        let output = segmenter::reassemble(
            &analysis.segments.iter().map(|s| s.segment.clone()).collect::<Vec<_>>(),
            &indices,
        );
        return Ok(CompressionResult {
            output,
            stats: CompressionStats {
                original_tokens,
                compressed_tokens: original_tokens,
                redundancy_removed: 0,
                boilerplate_removed: 0,
                extractive_removed: 0,
                segments_kept: segments_total,
                segments_total,
            },
        });
    }
    
    let mut stats = CompressionStats {
        original_tokens,
        compressed_tokens: 0,
        redundancy_removed: 0,
        boilerplate_removed: 0,
        extractive_removed: 0,
        segments_kept: 0,
        segments_total,
    };
    
    // Strategy 1: Redundancy removal
    if config.strategies.redundancy {
        stats.redundancy_removed = RedundancyRemover::remove_redundancy(
            &mut analysis.segments,
            config.thresholds.redundancy_similarity,
        );
    }
    
    // Strategy 2: Boilerplate filtering (already marked during analysis)
    if config.strategies.boilerplate {
        stats.boilerplate_removed = analysis.segments
            .iter()
            .filter(|s| s.is_boilerplate)
            .map(|s| s.segment.tokens)
            .sum();
    }
    
    // Calculate available budget
    let available = BudgetAllocator::calculate_available(budget, preserve_head, preserve_tail)?;
    
    // Strategy 3: Extractive compression - select best segments
    let selected = if config.strategies.extractive {
        ExtractiveCompressor::select(&analysis.segments, available, config)
    } else {
        // Just take everything that fits without extractive selection
        BudgetAllocator::fit_to_budget(&analysis.segments, available)
    };
    
    // Handle preserves
    let final_selection = if preserve_head > 0 || preserve_tail > 0 {
        BudgetAllocator::fit_with_preserves(
            &analysis.segments,
            budget,
            preserve_head,
            preserve_tail,
        )?
    } else {
        selected
    };
    
    // Calculate final stats
    stats.segments_kept = final_selection.len();
    stats.compressed_tokens = final_selection
        .iter()
        .filter_map(|&i| analysis.segments.get(i))
        .map(|s| s.segment.tokens)
        .sum();
    
    stats.extractive_removed = original_tokens
        .saturating_sub(stats.redundancy_removed)
        .saturating_sub(stats.boilerplate_removed)
        .saturating_sub(stats.compressed_tokens);
    
    // Reassemble output
    let segments: Vec<_> = analysis.segments.iter().map(|s| s.segment.clone()).collect();
    let output = segmenter::reassemble(&segments, &final_selection);
    
    Ok(CompressionResult { output, stats })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{AnalyzedSegment, MinHashSignature};
    use crate::segmenter::Segment;
    use crate::cli::Args;
    use crate::config;
    use clap::Parser;

    fn test_config() -> Config {
        let args = Args::parse_from(["test", "--budget", "1000"]);
        config::load(None).and_then(|c| config::merge(c, &args)).unwrap()
    }

    fn make_segment(index: usize, text: &str, tokens: usize, score: f64) -> AnalyzedSegment {
        AnalyzedSegment {
            index,
            segment: Segment {
                text: text.to_string(),
                start: 0,
                end: text.len(),
                tokens,
                is_code_block: false,
                heading_level: 0,
            },
            minhash: MinHashSignature::new(),
            tfidf_score: score,
            boilerplate_score: 0.0,
            importance_score: score,
            is_redundant: false,
            is_boilerplate: false,
            is_preserved: false,
        }
    }

    fn make_analysis(segments: Vec<AnalyzedSegment>) -> AnalysisResult {
        let total_tokens = segments.iter().map(|s| s.segment.tokens).sum();
        AnalysisResult {
            segments,
            total_tokens,
        }
    }

    #[test]
    fn test_compress_no_compression_needed() {
        let config = test_config();
        let analysis = make_analysis(vec![
            make_segment(0, "Hello world", 10, 0.5),
        ]);
        let result = compress(analysis, 1000, 0, 0, &config).unwrap();
        assert_eq!(result.stats.compressed_tokens, 10);
        assert_eq!(result.stats.original_tokens, 10);
        assert!(result.output.contains("Hello world"));
    }

    #[test]
    fn test_compress_empty_error() {
        let config = test_config();
        let analysis = AnalysisResult {
            segments: vec![],
            total_tokens: 0,
        };
        assert!(compress(analysis, 1000, 0, 0, &config).is_err());
    }

    #[test]
    fn test_compress_selects_best() {
        let config = test_config();
        let analysis = make_analysis(vec![
            make_segment(0, "Low score content", 50, 0.3),
            make_segment(1, "High score content", 50, 0.9),
        ]);
        let result = compress(analysis, 60, 0, 0, &config).unwrap();
        // Should prefer high score
        assert!(result.output.contains("High score"));
        assert!(result.stats.compressed_tokens <= 60);
    }

    #[test]
    fn test_compress_excludes_boilerplate() {
        let config = test_config();
        let mut segments = vec![
            make_segment(0, "MIT License boilerplate text here", 50, 0.9),
            make_segment(1, "Important real content here today", 50, 0.9),
        ];
        segments[0].is_boilerplate = true;
        let analysis = make_analysis(segments);
        
        // Budget is less than total (100) so compression happens
        let result = compress(analysis, 80, 0, 0, &config).unwrap();
        
        // Boilerplate tokens should be counted
        assert_eq!(result.stats.boilerplate_removed, 50);
    }

    #[test]
    fn test_compression_stats() {
        let config = test_config();
        let analysis = make_analysis(vec![
            make_segment(0, "Content one", 100, 0.5),
            make_segment(1, "Content two", 100, 0.6),
        ]);
        let result = compress(analysis, 150, 0, 0, &config).unwrap();
        
        assert_eq!(result.stats.original_tokens, 200);
        assert!(result.stats.compressed_tokens <= 150);
        assert!(result.stats.compression_ratio() <= 1.0);
    }

    #[test]
    fn test_compression_ratio() {
        let stats = CompressionStats {
            original_tokens: 1000,
            compressed_tokens: 500,
            ..Default::default()
        };
        assert!((stats.compression_ratio() - 0.5).abs() < 0.01);
        assert_eq!(stats.tokens_saved(), 500);
    }
}
