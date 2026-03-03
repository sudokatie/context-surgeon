use crate::analysis::AnalyzedSegment;
use crate::config::Config;
use regex::Regex;

pub struct ExtractiveCompressor;

impl ExtractiveCompressor {
    /// Compute importance score for a segment
    pub fn compute_importance(
        segment: &AnalyzedSegment,
        position: usize,
        total: usize,
        config: &Config,
    ) -> f64 {
        if total == 0 {
            return 0.0;
        }
        
        let mut score = 0.0;
        
        // Position weight (earlier is generally more important)
        let position_ratio = position as f64 / total as f64;
        let position_weight = config.thresholds.position_decay.powf(position_ratio * 10.0);
        score += position_weight * 0.3;
        
        // TF-IDF weight
        score += segment.tfidf_score * 0.4;
        
        // Structural weight (detect headings, lists)
        let structural = Self::compute_structural_weight(&segment.segment.text);
        score += structural * 0.2;
        
        // Length adjustment
        let length = segment.segment.tokens;
        let length_penalty = if length < 10 {
            -0.1
        } else if length > 500 {
            -0.05
        } else {
            0.0
        };
        score += length_penalty;
        
        // Preserve pattern boost
        if Self::matches_preserve_pattern(&segment.segment.text, &config.preserve_patterns) {
            score += 0.2;
        }
        
        // Boilerplate penalty
        if segment.is_boilerplate {
            score *= 0.1;
        }
        
        // Redundant penalty
        if segment.is_redundant {
            score *= 0.0;  // Completely exclude
        }
        
        score.clamp(0.0, 1.0)
    }
    
    fn compute_structural_weight(text: &str) -> f64 {
        let mut weight: f64 = 0.0;
        
        // Headings (markdown or plain)
        if text.starts_with('#') || text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && text.ends_with(':') {
            weight += 0.3;
        }
        
        // Lists
        if text.starts_with("- ") || text.starts_with("* ") || text.chars().next().map(|c| c.is_numeric()).unwrap_or(false) {
            weight += 0.2;
        }
        
        // Questions (often important)
        if text.contains('?') {
            weight += 0.1;
        }
        
        // Code blocks (high signal)
        if text.starts_with("```") || text.starts_with("    ") || text.contains("fn ") || text.contains("def ") {
            weight += 0.2;
        }
        
        // Important keywords
        let important_keywords = ["important", "note", "warning", "todo", "fixme", "critical", "key", "summary"];
        let lower = text.to_lowercase();
        for keyword in important_keywords {
            if lower.contains(keyword) {
                weight += 0.1;
                break;
            }
        }
        
        weight.min(1.0)
    }
    
    fn matches_preserve_pattern(text: &str, patterns: &[Regex]) -> bool {
        patterns.iter().any(|p| p.is_match(text))
    }
    
    /// Select segments to meet budget using importance scores
    pub fn select(
        segments: &[AnalyzedSegment],
        budget: usize,
        config: &Config,
    ) -> Vec<usize> {
        if segments.is_empty() || budget == 0 {
            return vec![];
        }
        
        let total = segments.len();
        
        // First, always include preserved segments (code blocks when --preserve-code)
        let mut selected = Vec::new();
        let mut used = 0;
        
        for (i, seg) in segments.iter().enumerate() {
            if seg.is_preserved {
                selected.push(i);
                used += seg.segment.tokens;
            }
        }
        
        // Compute importance for each non-preserved segment
        let mut scored: Vec<(usize, f64, usize)> = segments
            .iter()
            .enumerate()
            .filter(|(i, seg)| !seg.is_preserved && !selected.contains(i))
            .map(|(i, seg)| {
                let importance = Self::compute_importance(seg, i, total, config);
                (i, importance, seg.segment.tokens)
            })
            .filter(|(_, importance, _)| *importance > config.thresholds.importance_minimum)
            .collect();
        
        // Sort by importance descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Greedily select from remaining budget
        for (idx, _score, tokens) in scored {
            if used + tokens <= budget {
                selected.push(idx);
                used += tokens;
            }
        }
        
        // Return in original order
        selected.sort();
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::Segment;
    use crate::analysis::MinHashSignature;
    use crate::cli::Args;
    use crate::config;
    use clap::Parser;

    fn test_config() -> Config {
        let args = Args::parse_from(["test", "--budget", "1000"]);
        config::load(None).and_then(|c| config::merge(c, &args)).unwrap()
    }

    fn make_segment(index: usize, text: &str, tokens: usize, tfidf: f64) -> AnalyzedSegment {
        AnalyzedSegment {
            index,
            segment: Segment {
                text: text.to_string(),
                start: 0,
                end: text.len(),
                tokens,
                is_code_block: false,
            },
            minhash: MinHashSignature::new(),
            tfidf_score: tfidf,
            boilerplate_score: 0.0,
            importance_score: tfidf,
            is_redundant: false,
            is_boilerplate: false,
            is_preserved: false,
        }
    }

    #[test]
    fn test_compute_importance_position() {
        let config = test_config();
        let seg = make_segment(0, "test content", 10, 0.5);
        
        // Earlier positions should score higher
        let early = ExtractiveCompressor::compute_importance(&seg, 0, 10, &config);
        let late = ExtractiveCompressor::compute_importance(&seg, 9, 10, &config);
        assert!(early > late);
    }

    #[test]
    fn test_compute_importance_tfidf() {
        let config = test_config();
        let high_tfidf = make_segment(0, "unique rare terms", 10, 0.9);
        let low_tfidf = make_segment(1, "common words here", 10, 0.1);
        
        let high_score = ExtractiveCompressor::compute_importance(&high_tfidf, 5, 10, &config);
        let low_score = ExtractiveCompressor::compute_importance(&low_tfidf, 5, 10, &config);
        assert!(high_score > low_score);
    }

    #[test]
    fn test_compute_importance_boilerplate() {
        let config = test_config();
        let mut seg = make_segment(0, "MIT License", 10, 0.5);
        seg.is_boilerplate = true;
        
        let score = ExtractiveCompressor::compute_importance(&seg, 5, 10, &config);
        assert!(score < 0.2);  // Should be heavily penalized
    }

    #[test]
    fn test_compute_importance_redundant() {
        let config = test_config();
        let mut seg = make_segment(0, "duplicate content", 10, 0.5);
        seg.is_redundant = true;
        
        let score = ExtractiveCompressor::compute_importance(&seg, 5, 10, &config);
        assert_eq!(score, 0.0);  // Should be zero
    }

    #[test]
    fn test_structural_weight_heading() {
        let weight = ExtractiveCompressor::compute_structural_weight("# Important Section");
        assert!(weight > 0.0);
    }

    #[test]
    fn test_structural_weight_list() {
        let weight = ExtractiveCompressor::compute_structural_weight("- First item in list");
        assert!(weight > 0.0);
    }

    #[test]
    fn test_structural_weight_code() {
        let weight = ExtractiveCompressor::compute_structural_weight("def function():");
        assert!(weight > 0.0);
    }

    #[test]
    fn test_select_empty() {
        let config = test_config();
        let selected = ExtractiveCompressor::select(&[], 1000, &config);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_select_fits_all() {
        let config = test_config();
        let segments = vec![
            make_segment(0, "content one", 100, 0.5),
            make_segment(1, "content two", 100, 0.6),
        ];
        let selected = ExtractiveCompressor::select(&segments, 1000, &config);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_select_budget_constraint() {
        let config = test_config();
        let segments = vec![
            make_segment(0, "content one", 100, 0.5),
            make_segment(1, "content two", 100, 0.9),  // Higher score
        ];
        let selected = ExtractiveCompressor::select(&segments, 150, &config);
        // Should select higher scored one
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_select_original_order() {
        let config = test_config();
        let segments = vec![
            make_segment(0, "content one", 100, 0.5),
            make_segment(1, "content two", 100, 0.6),
            make_segment(2, "content three", 100, 0.7),
        ];
        let selected = ExtractiveCompressor::select(&segments, 250, &config);
        // Should be in original order
        assert!(selected.windows(2).all(|w| w[0] < w[1]));
    }
}
