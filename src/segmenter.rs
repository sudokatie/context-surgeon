use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    #[allow(dead_code)]
    pub start: usize,
    #[allow(dead_code)]
    pub end: usize,
    pub tokens: usize,
    /// Whether this segment is a code block (fenced with ```)
    pub is_code_block: bool,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SegmentMode {
    Paragraph,
    Sentence,
    Line,
}

const MIN_SEGMENT_SIZE: usize = 10;
const MAX_SEGMENT_SIZE: usize = 10_000;

pub fn segment(text: &str, mode: SegmentMode) -> Vec<Segment> {
    match mode {
        SegmentMode::Paragraph => segment_paragraphs(text),
        SegmentMode::Sentence => segment_sentences(text),
        SegmentMode::Line => segment_lines(text),
    }
}

/// Segment text with code block detection
/// Code blocks (fenced with ```) are kept as single segments and marked
pub fn segment_with_code_blocks(text: &str, mode: SegmentMode) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut pos = 0;
    
    // Find all code blocks first
    let code_blocks = find_code_blocks(text);
    
    for (block_start, block_end) in &code_blocks {
        // Process text before this code block
        if pos < *block_start {
            let before_text = &text[pos..*block_start];
            let before_segments = segment(before_text, mode);
            for mut seg in before_segments {
                seg.start += pos;
                seg.end += pos;
                seg.is_code_block = false;
                segments.push(seg);
            }
        }
        
        // Add the code block as a single preserved segment
        let code_text = &text[*block_start..*block_end];
        if code_text.len() >= MIN_SEGMENT_SIZE {
            segments.push(Segment {
                text: code_text.to_string(),
                start: *block_start,
                end: *block_end,
                tokens: 0,
                is_code_block: true,
            });
        }
        
        pos = *block_end;
    }
    
    // Process remaining text after last code block
    if pos < text.len() {
        let after_text = &text[pos..];
        let after_segments = segment(after_text, mode);
        for mut seg in after_segments {
            seg.start += pos;
            seg.end += pos;
            seg.is_code_block = false;
            segments.push(seg);
        }
    }
    
    // Sort by start position to maintain order
    segments.sort_by_key(|s| s.start);
    
    segments
}

/// Find fenced code blocks (```...```) in text
/// Returns list of (start, end) byte positions
fn find_code_blocks(text: &str) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut pos = 0;
    
    while let Some(start) = text[pos..].find("```") {
        let block_start = pos + start;
        
        // Find end of opening fence (includes language specifier)
        let fence_end = text[block_start..].find('\n')
            .map(|p| block_start + p + 1)
            .unwrap_or(block_start + 3);
        
        // Find closing fence
        if let Some(close_offset) = text[fence_end..].find("\n```") {
            let block_end = fence_end + close_offset + 4; // Include closing ```
            
            // Check if there's more after closing fence (like language or newline)
            let final_end = if block_end < text.len() && text[block_end..].starts_with('\n') {
                block_end + 1
            } else {
                block_end
            };
            
            blocks.push((block_start, final_end.min(text.len())));
            pos = final_end;
        } else {
            // No closing fence, skip this opening
            pos = fence_end;
        }
    }
    
    blocks
}

/// Check if a string is a fenced code block
pub fn is_fenced_code_block(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("```") && trimmed.ends_with("```") && trimmed.len() > 6
}

fn segment_paragraphs(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_start = 0;
    
    for part in text.split("\n\n") {
        let trimmed = part.trim();
        if trimmed.len() >= MIN_SEGMENT_SIZE {
            let start = text[current_start..].find(trimmed).map(|i| current_start + i).unwrap_or(current_start);
            let end = start + trimmed.len();
            
            if trimmed.len() <= MAX_SEGMENT_SIZE {
                segments.push(Segment {
                    text: trimmed.to_string(),
                    start,
                    end,
                    tokens: 0,
                    is_code_block: false,
                });
            } else {
                // Split large segments
                segments.extend(split_large_segment(trimmed, start));
            }
        }
        current_start += part.len() + 2;  // +2 for \n\n
    }
    
    segments
}

fn segment_sentences(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut last_end = 0;
    
    for sentence in text.unicode_sentences() {
        let trimmed = sentence.trim();
        if trimmed.len() >= MIN_SEGMENT_SIZE {
            let start = text[last_end..].find(trimmed).map(|i| last_end + i).unwrap_or(last_end);
            let end = start + trimmed.len();
            
            segments.push(Segment {
                text: trimmed.to_string(),
                start,
                end,
                tokens: 0,
                is_code_block: false,
            });
            
            last_end = end;
        }
    }
    
    segments
}

fn segment_lines(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_pos = 0;
    
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.len() >= MIN_SEGMENT_SIZE {
            segments.push(Segment {
                text: trimmed.to_string(),
                start: current_pos,
                end: current_pos + line.len(),
                tokens: 0,
                is_code_block: false,
            });
        }
        current_pos += line.len() + 1;  // +1 for newline
    }
    
    segments
}

fn split_large_segment(text: &str, base_start: usize) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut remaining = text;
    let mut offset = 0;
    
    while remaining.len() > MAX_SEGMENT_SIZE {
        // Find a good break point (sentence end or space)
        let break_point = find_break_point(&remaining[..MAX_SEGMENT_SIZE]);
        let chunk = &remaining[..break_point];
        
        segments.push(Segment {
            text: chunk.to_string(),
            start: base_start + offset,
            end: base_start + offset + chunk.len(),
            tokens: 0,
            is_code_block: false,
        });
        
        remaining = remaining[break_point..].trim_start();
        offset += break_point;
    }
    
    if remaining.len() >= MIN_SEGMENT_SIZE {
        segments.push(Segment {
            text: remaining.to_string(),
            start: base_start + offset,
            end: base_start + offset + remaining.len(),
            tokens: 0,
            is_code_block: false,
        });
    }
    
    segments
}

fn find_break_point(text: &str) -> usize {
    // Try to find sentence end
    if let Some(pos) = text.rfind(". ") {
        return pos + 2;
    }
    if let Some(pos) = text.rfind(".\n") {
        return pos + 2;
    }
    // Fall back to last space
    if let Some(pos) = text.rfind(' ') {
        return pos + 1;
    }
    // Last resort: just split
    text.len()
}

pub fn reassemble(segments: &[Segment], indices: &[usize]) -> String {
    let mut sorted_indices: Vec<_> = indices.to_vec();
    sorted_indices.sort();
    
    let mut result = String::new();
    
    for &idx in &sorted_indices {
        if idx < segments.len() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(&segments[idx].text);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_split() {
        let text = "First paragraph here.\n\nSecond paragraph here.";
        let segments = segment(text, SegmentMode::Paragraph);
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_sentence_split() {
        let text = "First sentence here. Second sentence here. Third one.";
        let segments = segment(text, SegmentMode::Sentence);
        assert!(segments.len() >= 2);
    }

    #[test]
    fn test_line_split() {
        let text = "First line here\nSecond line here\nThird line here";
        let segments = segment(text, SegmentMode::Line);
        assert_eq!(segments.len(), 3);
    }

    #[test]
    fn test_minimum_size() {
        let text = "Hi\n\nThis is a longer segment.";
        let segments = segment(text, SegmentMode::Paragraph);
        assert_eq!(segments.len(), 1);  // "Hi" is too short
    }

    #[test]
    fn test_empty_input() {
        let segments = segment("", SegmentMode::Paragraph);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_unicode() {
        let text = "First paragraph with unicode: 你好\n\nSecond paragraph: мир";
        let segments = segment(text, SegmentMode::Paragraph);
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_reassemble() {
        let segments = vec![
            Segment { text: "First".to_string(), start: 0, end: 5, tokens: 0, is_code_block: false },
            Segment { text: "Second".to_string(), start: 7, end: 13, tokens: 0, is_code_block: false },
            Segment { text: "Third".to_string(), start: 15, end: 20, tokens: 0, is_code_block: false },
        ];
        let result = reassemble(&segments, &[2, 0]);  // Out of order
        assert!(result.contains("First"));
        assert!(result.contains("Third"));
        assert!(!result.contains("Second"));
    }

    #[test]
    fn test_reassemble_order() {
        let segments = vec![
            Segment { text: "First".to_string(), start: 0, end: 5, tokens: 0, is_code_block: false },
            Segment { text: "Second".to_string(), start: 7, end: 13, tokens: 0, is_code_block: false },
        ];
        let result = reassemble(&segments, &[1, 0]);
        // Should be in original order (0 before 1)
        assert!(result.starts_with("First"));
    }

    #[test]
    fn test_whitespace_handling() {
        let text = "   Paragraph with whitespace   \n\n   Another one   ";
        let segments = segment(text, SegmentMode::Paragraph);
        assert_eq!(segments.len(), 2);
        assert!(!segments[0].text.starts_with(' '));
    }

    #[test]
    fn test_single_segment() {
        let text = "Just one paragraph without any breaks.";
        let segments = segment(text, SegmentMode::Paragraph);
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn test_mixed_newlines() {
        let text = "Para one.\r\n\r\nPara two.\n\nPara three.";
        let segments = segment(text, SegmentMode::Paragraph);
        assert!(segments.len() >= 2);
    }

    #[test]
    fn test_find_code_blocks() {
        let text = "Before\n\n```rust\nfn main() {}\n```\n\nAfter";
        let blocks = find_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(text[blocks[0].0..blocks[0].1].starts_with("```"));
    }

    #[test]
    fn test_find_multiple_code_blocks() {
        let text = "```python\nprint('hi')\n```\n\nText\n\n```js\nconsole.log('hi')\n```";
        let blocks = find_code_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_is_fenced_code_block() {
        assert!(is_fenced_code_block("```\ncode\n```"));
        assert!(is_fenced_code_block("```rust\nfn main() {}\n```"));
        assert!(!is_fenced_code_block("not code"));
        assert!(!is_fenced_code_block("```")); // Too short
    }

    #[test]
    fn test_segment_with_code_blocks() {
        let text = "Introduction paragraph here.\n\n```python\ndef hello():\n    print('hi')\n```\n\nConclusion paragraph here.";
        let segments = segment_with_code_blocks(text, SegmentMode::Paragraph);
        
        // Should have: intro, code block, conclusion
        assert!(segments.len() >= 2);
        
        // Find the code block segment
        let code_seg = segments.iter().find(|s| s.is_code_block);
        assert!(code_seg.is_some(), "Should have a code block segment");
        assert!(code_seg.unwrap().text.contains("def hello"));
    }

    #[test]
    fn test_segment_with_code_blocks_preserves_order() {
        let text = "First para.\n\n```\ncode\n```\n\nSecond para.";
        let segments = segment_with_code_blocks(text, SegmentMode::Paragraph);
        
        // Verify order: segments should be sorted by start position
        for i in 1..segments.len() {
            assert!(segments[i].start >= segments[i-1].start);
        }
    }

    #[test]
    fn test_segment_code_block_marked() {
        let text = "Normal text here.\n\n```js\nconst x = 1;\n```\n\nMore normal text.";
        let segments = segment_with_code_blocks(text, SegmentMode::Paragraph);
        
        let code_count = segments.iter().filter(|s| s.is_code_block).count();
        let normal_count = segments.iter().filter(|s| !s.is_code_block).count();
        
        assert_eq!(code_count, 1);
        assert!(normal_count >= 1);
    }
}
