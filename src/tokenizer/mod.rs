mod openai;
mod anthropic;
mod approximate;

pub use openai::OpenAITokenizer;
pub use anthropic::AnthropicTokenizer;
pub use approximate::ApproximateTokenizer;

use crate::cli::TokenizerKind;
use crate::segmenter::Segment;

pub trait Tokenizer: Send + Sync {
    fn count(&self, text: &str) -> usize;
    
    fn count_segments(&self, segments: &mut [Segment]) {
        for segment in segments {
            segment.tokens = self.count(&segment.text);
        }
    }
}

pub fn create(kind: &TokenizerKind) -> Box<dyn Tokenizer> {
    match kind {
        TokenizerKind::Openai => Box::new(OpenAITokenizer::new()),
        TokenizerKind::Anthropic => Box::new(AnthropicTokenizer),
        TokenizerKind::Approximate => Box::new(ApproximateTokenizer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_openai() {
        let t = create(&TokenizerKind::Openai);
        assert!(t.count("hello world") > 0);
    }

    #[test]
    fn test_create_anthropic() {
        let t = create(&TokenizerKind::Anthropic);
        assert!(t.count("hello world") > 0);
    }

    #[test]
    fn test_create_approximate() {
        let t = create(&TokenizerKind::Approximate);
        assert!(t.count("hello world") > 0);
    }

    #[test]
    fn test_count_segments() {
        let t = create(&TokenizerKind::Approximate);
        let mut segments = vec![
            Segment { text: "hello".to_string(), start: 0, end: 5, tokens: 0 },
            Segment { text: "world".to_string(), start: 6, end: 11, tokens: 0 },
        ];
        t.count_segments(&mut segments);
        assert!(segments[0].tokens > 0);
        assert!(segments[1].tokens > 0);
    }
}
