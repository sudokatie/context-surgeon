use crate::analysis::AnalyzedSegment;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BudgetError {
    #[error("preserved tokens ({preserved}) exceed budget ({budget})")]
    PreserveExceedsBudget { preserved: usize, budget: usize },
    #[error("budget cannot be zero")]
    ZeroBudget,
}

pub struct BudgetAllocator;

impl BudgetAllocator {
    pub fn calculate_available(
        total_budget: usize,
        preserve_head: usize,
        preserve_tail: usize,
    ) -> Result<usize, BudgetError> {
        if total_budget == 0 {
            return Err(BudgetError::ZeroBudget);
        }
        
        let preserved = preserve_head + preserve_tail;
        if preserved > total_budget {
            return Err(BudgetError::PreserveExceedsBudget {
                preserved,
                budget: total_budget,
            });
        }
        Ok(total_budget - preserved)
    }
    
    pub fn fit_to_budget(
        segments: &[AnalyzedSegment],
        budget: usize,
    ) -> Vec<usize> {
        if segments.is_empty() || budget == 0 {
            return vec![];
        }
        
        // Create (index, score, tokens) tuples, excluding redundant/boilerplate
        let mut scored: Vec<_> = segments
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_redundant && !s.is_boilerplate)
            .map(|(i, s)| (i, s.importance_score, s.segment.tokens))
            .collect();
        
        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Greedily select highest-scoring segments that fit
        let mut selected = Vec::new();
        let mut used = 0;
        
        for (idx, _score, tokens) in scored {
            if used + tokens <= budget {
                selected.push(idx);
                used += tokens;
            }
        }
        
        // Return in original order for coherent output
        selected.sort();
        selected
    }
    
    pub fn fit_with_preserves(
        segments: &[AnalyzedSegment],
        budget: usize,
        preserve_head: usize,
        preserve_tail: usize,
    ) -> Result<Vec<usize>, BudgetError> {
        let available = Self::calculate_available(budget, preserve_head, preserve_tail)?;
        
        if segments.is_empty() {
            return Ok(vec![]);
        }
        
        // Always include preserved segments
        let mut selected: Vec<usize> = Vec::new();
        let mut used = 0;
        
        // Find head segments to preserve
        let mut head_tokens = 0;
        for (i, seg) in segments.iter().enumerate() {
            if head_tokens >= preserve_head {
                break;
            }
            selected.push(i);
            head_tokens += seg.segment.tokens;
            used += seg.segment.tokens;
        }
        
        // Find tail segments to preserve
        let mut tail_tokens = 0;
        let mut tail_indices: Vec<usize> = Vec::new();
        for (i, seg) in segments.iter().enumerate().rev() {
            if tail_tokens >= preserve_tail {
                break;
            }
            if !selected.contains(&i) {
                tail_indices.push(i);
                tail_tokens += seg.segment.tokens;
            }
        }
        
        // Add tail indices and count tokens
        for &i in &tail_indices {
            selected.push(i);
            used += segments[i].segment.tokens;
        }
        
        // Fill remaining budget with highest-scoring middle segments
        let middle_budget = if used < budget { budget - used } else { 0 };
        
        let mut middle_scored: Vec<_> = segments
            .iter()
            .enumerate()
            .filter(|(i, s)| !selected.contains(i) && !s.is_redundant && !s.is_boilerplate)
            .map(|(i, s)| (i, s.importance_score, s.segment.tokens))
            .collect();
        
        middle_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut middle_used = 0;
        for (idx, _score, tokens) in middle_scored {
            if middle_used + tokens <= middle_budget {
                selected.push(idx);
                middle_used += tokens;
            }
        }
        
        selected.sort();
        Ok(selected)
    }
    
    pub fn verify(segments: &[AnalyzedSegment], selected: &[usize], budget: usize) -> bool {
        let total: usize = selected
            .iter()
            .filter_map(|&i| segments.get(i))
            .map(|s| s.segment.tokens)
            .sum();
        total <= budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::Segment;
    use crate::analysis::MinHashSignature;

    fn make_segment(index: usize, tokens: usize, score: f64) -> AnalyzedSegment {
        AnalyzedSegment {
            index,
            segment: Segment {
                text: format!("segment {}", index),
                start: 0,
                end: 10,
                tokens,
            },
            minhash: MinHashSignature::new(),
            tfidf_score: score,
            boilerplate_score: 0.0,
            importance_score: score,
            is_redundant: false,
            is_boilerplate: false,
        }
    }

    #[test]
    fn test_calculate_available() {
        assert_eq!(BudgetAllocator::calculate_available(1000, 100, 100).unwrap(), 800);
        assert_eq!(BudgetAllocator::calculate_available(1000, 0, 0).unwrap(), 1000);
    }

    #[test]
    fn test_calculate_available_error() {
        assert!(BudgetAllocator::calculate_available(100, 50, 60).is_err());
        assert!(BudgetAllocator::calculate_available(0, 0, 0).is_err());
    }

    #[test]
    fn test_fit_to_budget_empty() {
        let selected = BudgetAllocator::fit_to_budget(&[], 1000);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_fit_to_budget_all_fit() {
        let segments = vec![
            make_segment(0, 100, 0.5),
            make_segment(1, 100, 0.8),
            make_segment(2, 100, 0.3),
        ];
        let selected = BudgetAllocator::fit_to_budget(&segments, 1000);
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_fit_to_budget_selects_best() {
        let segments = vec![
            make_segment(0, 100, 0.3),
            make_segment(1, 100, 0.9),  // Highest score
            make_segment(2, 100, 0.5),
        ];
        let selected = BudgetAllocator::fit_to_budget(&segments, 150);
        // Should select index 1 (highest score)
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_fit_to_budget_excludes_redundant() {
        let mut segments = vec![
            make_segment(0, 100, 0.9),
            make_segment(1, 100, 0.8),
        ];
        segments[0].is_redundant = true;
        
        let selected = BudgetAllocator::fit_to_budget(&segments, 150);
        assert!(!selected.contains(&0));
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_fit_to_budget_excludes_boilerplate() {
        let mut segments = vec![
            make_segment(0, 100, 0.9),
            make_segment(1, 100, 0.8),
        ];
        segments[0].is_boilerplate = true;
        
        let selected = BudgetAllocator::fit_to_budget(&segments, 150);
        assert!(!selected.contains(&0));
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_fit_to_budget_original_order() {
        let segments = vec![
            make_segment(0, 100, 0.3),
            make_segment(1, 100, 0.9),
            make_segment(2, 100, 0.5),
        ];
        let selected = BudgetAllocator::fit_to_budget(&segments, 250);
        // Should be in order: 1, 2 (sorted by index)
        assert!(selected.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn test_fit_with_preserves() {
        let segments = vec![
            make_segment(0, 50, 0.1),   // Head
            make_segment(1, 100, 0.9),  // Middle - high score
            make_segment(2, 100, 0.2),  // Middle - low score
            make_segment(3, 50, 0.1),   // Tail
        ];
        let selected = BudgetAllocator::fit_with_preserves(&segments, 200, 50, 50).unwrap();
        
        // Should include head (0) and tail (3)
        assert!(selected.contains(&0));
        assert!(selected.contains(&3));
    }

    #[test]
    fn test_verify_within_budget() {
        let segments = vec![
            make_segment(0, 100, 0.5),
            make_segment(1, 100, 0.5),
        ];
        assert!(BudgetAllocator::verify(&segments, &[0, 1], 200));
        assert!(!BudgetAllocator::verify(&segments, &[0, 1], 150));
    }
}
