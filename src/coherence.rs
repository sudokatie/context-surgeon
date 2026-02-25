use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Orphaned pronouns - pronouns that might lack referents
    static ref ORPHANED_PRONOUNS: Regex = Regex::new(
        r"(?i)^(he|she|it|they|this|that|these|those)\s"
    ).unwrap();
    
    // Dangling references
    static ref DANGLING_REFS: Regex = Regex::new(
        r"(?i)(as (mentioned|noted|described|discussed|shown|seen) (above|earlier|previously|before))"
    ).unwrap();
    
    // Incomplete lists
    static ref LIST_CONTINUATION: Regex = Regex::new(
        r"(?i)^(\d+\.|[-*•])\s"
    ).unwrap();
    
    // Code block indicators without context
    static ref CODE_REFERENCE: Regex = Regex::new(
        r"(?i)(the (above|following|below) (code|example|snippet|function|method))"
    ).unwrap();
}

#[derive(Debug, Clone)]
pub struct CoherenceWarning {
    pub kind: WarningKind,
    #[allow(dead_code)]
    pub segment_index: usize,
    pub text_preview: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarningKind {
    OrphanedPronoun,
    DanglingReference,
    IncompleteList,
    OrphanedCodeReference,
}

impl std::fmt::Display for WarningKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarningKind::OrphanedPronoun => write!(f, "orphaned pronoun"),
            WarningKind::DanglingReference => write!(f, "dangling reference"),
            WarningKind::IncompleteList => write!(f, "incomplete list"),
            WarningKind::OrphanedCodeReference => write!(f, "orphaned code reference"),
        }
    }
}

pub struct CoherenceChecker;

impl CoherenceChecker {
    pub fn check(segments: &[String], kept_indices: &[usize]) -> Vec<CoherenceWarning> {
        let mut warnings = Vec::new();
        
        // Build the kept segment texts in order
        let mut sorted_indices: Vec<usize> = kept_indices.to_vec();
        sorted_indices.sort();
        
        for (position, &idx) in sorted_indices.iter().enumerate() {
            if idx >= segments.len() {
                continue;
            }
            
            let text = &segments[idx];
            let preview = truncate_preview(text, 50);
            
            // Check for gaps: either first kept segment doesn't start at 0,
            // or there's a gap between this and the previous kept segment
            let has_gap_before = if position == 0 {
                idx > 0  // First kept segment, but segments before it were skipped
            } else {
                let prev_kept_idx = sorted_indices[position - 1];
                idx > prev_kept_idx + 1  // Gap between kept segments
            };
            
            if has_gap_before && ORPHANED_PRONOUNS.is_match(text) {
                warnings.push(CoherenceWarning {
                    kind: WarningKind::OrphanedPronoun,
                    segment_index: idx,
                    text_preview: preview.clone(),
                });
            }
            
            // Check for dangling references
            if DANGLING_REFS.is_match(text) && has_gap_before {
                warnings.push(CoherenceWarning {
                    kind: WarningKind::DanglingReference,
                    segment_index: idx,
                    text_preview: preview.clone(),
                });
            }
            
            // Check for incomplete lists (list item without preceding context)
            if has_gap_before && LIST_CONTINUATION.is_match(text) {
                warnings.push(CoherenceWarning {
                    kind: WarningKind::IncompleteList,
                    segment_index: idx,
                    text_preview: preview.clone(),
                });
            }
            
            // Check for orphaned code references
            if (DANGLING_REFS.is_match(text) || CODE_REFERENCE.is_match(text))
                && has_gap_before {
                    warnings.push(CoherenceWarning {
                        kind: WarningKind::OrphanedCodeReference,
                        segment_index: idx,
                        text_preview: preview,
                    });
                }
        }
        
        warnings
    }
    
    pub fn print_warnings(warnings: &[CoherenceWarning]) {
        if warnings.is_empty() {
            return;
        }
        
        eprintln!("\ncoherence warnings ({}):", warnings.len());
        for w in warnings {
            eprintln!("  - {}: \"{}...\"", w.kind, w.text_preview);
        }
    }
}

fn truncate_preview(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        trimmed[..max_len].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orphaned_pronoun_first_segment() {
        // When first segment with pronoun is kept but there was context before it
        let segments = vec![
            "John was at the mall shopping for groceries.".to_string(),
            "He walked to the store nearby to buy milk.".to_string(),
            "The store was closed for renovation work.".to_string(),
        ];
        // Skip first segment (the context for "He")
        let warnings = CoherenceChecker::check(&segments, &[1, 2]);
        assert!(warnings.iter().any(|w| w.kind == WarningKind::OrphanedPronoun));
    }

    #[test]
    fn test_orphaned_pronoun_after_gap() {
        let segments = vec![
            "John was tired.".to_string(),
            "He went home.".to_string(),
            "It was late.".to_string(),
        ];
        // Skip segment 1, so "It was late" comes after a gap
        let warnings = CoherenceChecker::check(&segments, &[0, 2]);
        assert!(warnings.iter().any(|w| w.kind == WarningKind::OrphanedPronoun));
    }

    #[test]
    fn test_no_orphan_when_continuous() {
        let segments = vec![
            "John was tired after work today.".to_string(),
            "He went home to rest for a while.".to_string(),
        ];
        let warnings = CoherenceChecker::check(&segments, &[0, 1]);
        // "He" follows "John" continuously, so no orphan warning
        let orphan_count = warnings.iter()
            .filter(|w| w.kind == WarningKind::OrphanedPronoun)
            .count();
        assert_eq!(orphan_count, 0);
    }

    #[test]
    fn test_dangling_reference() {
        let segments = vec![
            "Introduction paragraph.".to_string(),
            "As mentioned above, this is important.".to_string(),
        ];
        // Skip first segment
        let warnings = CoherenceChecker::check(&segments, &[1]);
        assert!(warnings.iter().any(|w| w.kind == WarningKind::DanglingReference));
    }

    #[test]
    fn test_incomplete_list() {
        let segments = vec![
            "Here is a list of items to consider in your work".to_string(),
            "1. First item in the numbered list here".to_string(),
            "2. Second item in the numbered list here".to_string(),
        ];
        // Skip intro (index 0), keep list items (indices 1, 2)
        // Index 1 starts after a gap (0 was skipped)
        let warnings = CoherenceChecker::check(&segments, &[1, 2]);
        assert!(warnings.iter().any(|w| w.kind == WarningKind::IncompleteList));
    }

    #[test]
    fn test_code_reference() {
        let segments = vec![
            "Here is some example code to review".to_string(),
            "fn main() { println!(\"hello\"); }".to_string(),
            "As mentioned above, the code prints hello".to_string(),
        ];
        // Skip intro and code (indices 0, 1)
        // Index 2 comes after gap
        let warnings = CoherenceChecker::check(&segments, &[2]);
        assert!(warnings.iter().any(|w| w.kind == WarningKind::DanglingReference));
    }

    #[test]
    fn test_empty_input() {
        let segments: Vec<String> = vec![];
        let warnings = CoherenceChecker::check(&segments, &[]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_warning_display() {
        assert_eq!(format!("{}", WarningKind::OrphanedPronoun), "orphaned pronoun");
        assert_eq!(format!("{}", WarningKind::DanglingReference), "dangling reference");
    }

    #[test]
    fn test_truncate_preview() {
        let short = "short text";
        assert_eq!(truncate_preview(short, 50), "short text");
        
        let long = "this is a very long text that should be truncated at some point";
        let preview = truncate_preview(long, 20);
        assert_eq!(preview.len(), 20);
    }
}
