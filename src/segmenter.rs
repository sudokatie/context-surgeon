use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub tokens: usize,
}

#[derive(Debug, Clone, Copy)]
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
            Segment { text: "First".to_string(), start: 0, end: 5, tokens: 0 },
            Segment { text: "Second".to_string(), start: 7, end: 13, tokens: 0 },
            Segment { text: "Third".to_string(), start: 15, end: 20, tokens: 0 },
        ];
        let result = reassemble(&segments, &[2, 0]);  // Out of order
        assert!(result.contains("First"));
        assert!(result.contains("Third"));
        assert!(!result.contains("Second"));
    }

    #[test]
    fn test_reassemble_order() {
        let segments = vec![
            Segment { text: "First".to_string(), start: 0, end: 5, tokens: 0 },
            Segment { text: "Second".to_string(), start: 7, end: 13, tokens: 0 },
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
}
