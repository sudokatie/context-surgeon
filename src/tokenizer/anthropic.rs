use super::Tokenizer;

pub struct AnthropicTokenizer;

impl Tokenizer for AnthropicTokenizer {
    fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        
        let chars = text.chars().count();
        let words = text.split_whitespace().count();
        
        // tokens ≈ max(words * 1.3, chars / 4)
        let word_estimate = ((words as f64) * 1.3).ceil() as usize;
        let char_estimate = chars / 4;
        
        word_estimate.max(char_estimate).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let t = AnthropicTokenizer;
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_single_word() {
        let t = AnthropicTokenizer;
        let count = t.count("hello");
        assert!(count >= 1 && count <= 3);
    }

    #[test]
    fn test_sentence() {
        let t = AnthropicTokenizer;
        let count = t.count("The quick brown fox jumps over the lazy dog");
        assert!(count >= 9);  // 9 words * 1.3 ≈ 12
    }

    #[test]
    fn test_dense_text() {
        let t = AnthropicTokenizer;
        // No spaces, should use char estimate
        let count = t.count("abcdefghijklmnopqrstuvwxyz");  // 26 chars
        assert!(count >= 6);  // 26 / 4 = 6
    }

    #[test]
    fn test_minimum_one() {
        let t = AnthropicTokenizer;
        assert!(t.count("x") >= 1);
    }

    #[test]
    fn test_cjk() {
        let t = AnthropicTokenizer;
        // CJK characters are dense
        let count = t.count("你好世界");  // 4 chars, no spaces
        assert!(count >= 1);
    }

    #[test]
    fn test_mixed() {
        let t = AnthropicTokenizer;
        let count = t.count("Hello 世界!");
        assert!(count >= 2);
    }

    #[test]
    fn test_consistency() {
        let t = AnthropicTokenizer;
        let text = "test text for consistency";
        assert_eq!(t.count(text), t.count(text));
    }
}
