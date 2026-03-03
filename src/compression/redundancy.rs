use crate::analysis::AnalyzedSegment;

pub struct RedundancyRemover;

impl RedundancyRemover {
    /// Remove redundant segments, keeping only the best representative from each cluster.
    /// Returns the number of tokens removed.
    pub fn remove_redundancy(
        segments: &mut [AnalyzedSegment],
        threshold: f64,
    ) -> usize {
        use crate::analysis::MinHash;
        
        if segments.is_empty() {
            return 0;
        }
        
        // Get signatures and scores
        let signatures: Vec<_> = segments.iter().map(|s| s.minhash.clone()).collect();
        let scores: Vec<f64> = segments.iter().map(|s| s.importance_score).collect();
        
        // Find clusters
        let cluster_reps = MinHash::find_clusters(&signatures, &scores, threshold);
        
        // Mark non-representatives as redundant
        let mut removed_tokens = 0;
        for (i, &rep) in cluster_reps.iter().enumerate() {
            if rep != i && !segments[i].is_redundant {
                segments[i].is_redundant = true;
                removed_tokens += segments[i].segment.tokens;
            }
        }
        
        removed_tokens
    }
    
    /// Get indices of non-redundant segments
    #[allow(dead_code)]
    pub fn get_unique_indices(segments: &[AnalyzedSegment]) -> Vec<usize> {
        segments
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_redundant)
            .map(|(i, _)| i)
            .collect()
    }
    
    /// Count redundant segments
    #[allow(dead_code)]
    pub fn count_redundant(segments: &[AnalyzedSegment]) -> usize {
        segments.iter().filter(|s| s.is_redundant).count()
    }
    
    /// Count tokens in redundant segments
    #[allow(dead_code)]
    pub fn count_redundant_tokens(segments: &[AnalyzedSegment]) -> usize {
        segments
            .iter()
            .filter(|s| s.is_redundant)
            .map(|s| s.segment.tokens)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::Segment;
    use crate::analysis::{MinHash, MinHashSignature};

    fn make_segment(index: usize, text: &str, tokens: usize, score: f64) -> AnalyzedSegment {
        AnalyzedSegment {
            index,
            segment: Segment {
                text: text.to_string(),
                start: 0,
                end: text.len(),
                tokens,
                is_code_block: false,
            },
            minhash: MinHash::compute_signature(text),
            tfidf_score: score,
            boilerplate_score: 0.0,
            importance_score: score,
            is_redundant: false,
            is_boilerplate: false,
            is_preserved: false,
        }
    }

    #[test]
    fn test_remove_redundancy_empty() {
        let mut segments: Vec<AnalyzedSegment> = vec![];
        let removed = RedundancyRemover::remove_redundancy(&mut segments, 0.8);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_remove_redundancy_no_duplicates() {
        let mut segments = vec![
            make_segment(0, "The quick brown fox", 10, 0.5),
            make_segment(1, "Lorem ipsum dolor sit", 10, 0.5),
        ];
        let removed = RedundancyRemover::remove_redundancy(&mut segments, 0.8);
        assert_eq!(removed, 0);
        assert!(!segments[0].is_redundant);
        assert!(!segments[1].is_redundant);
    }

    #[test]
    fn test_remove_redundancy_identical() {
        let mut segments = vec![
            make_segment(0, "The quick brown fox jumps over", 10, 0.5),
            make_segment(1, "The quick brown fox jumps over", 10, 0.8),  // Higher score
        ];
        let removed = RedundancyRemover::remove_redundancy(&mut segments, 0.8);
        // One should be marked redundant
        let redundant_count = segments.iter().filter(|s| s.is_redundant).count();
        assert_eq!(redundant_count, 1);
        // The higher scored one (index 1) should NOT be redundant
        assert!(!segments[1].is_redundant);
        assert_eq!(removed, 10);
    }

    #[test]
    fn test_get_unique_indices() {
        let mut segments = vec![
            make_segment(0, "text one", 10, 0.5),
            make_segment(1, "text two", 10, 0.5),
        ];
        segments[1].is_redundant = true;
        
        let unique = RedundancyRemover::get_unique_indices(&segments);
        assert_eq!(unique, vec![0]);
    }

    #[test]
    fn test_count_redundant() {
        let mut segments = vec![
            make_segment(0, "text one", 10, 0.5),
            make_segment(1, "text two", 20, 0.5),
            make_segment(2, "text three", 30, 0.5),
        ];
        segments[1].is_redundant = true;
        segments[2].is_redundant = true;
        
        assert_eq!(RedundancyRemover::count_redundant(&segments), 2);
        assert_eq!(RedundancyRemover::count_redundant_tokens(&segments), 50);
    }

    #[test]
    fn test_keeps_higher_score() {
        let mut segments = vec![
            make_segment(0, "The quick brown fox jumps", 10, 0.3),
            make_segment(1, "The quick brown fox jumps", 10, 0.9),  // Higher score
            make_segment(2, "The quick brown fox jumps", 10, 0.5),
        ];
        RedundancyRemover::remove_redundancy(&mut segments, 0.8);
        
        // Index 1 (highest score) should not be redundant
        assert!(!segments[1].is_redundant);
    }
}
