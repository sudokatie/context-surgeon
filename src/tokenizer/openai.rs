use super::Tokenizer;
use lazy_static::lazy_static;
use tiktoken_rs::cl100k_base;

lazy_static! {
    static ref ENCODING: tiktoken_rs::CoreBPE = cl100k_base().unwrap();
}

pub struct OpenAITokenizer;

impl OpenAITokenizer {
    pub fn new() -> Self {
        // Touch encoding to ensure it's loaded
        let _ = &*ENCODING;
        Self
    }
}

impl Default for OpenAITokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for OpenAITokenizer {
    fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        ENCODING.encode_ordinary(text).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let t = OpenAITokenizer::new();
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_single_word() {
        let t = OpenAITokenizer::new();
        let count = t.count("hello");
        assert!(count > 0 && count < 5);
    }

    #[test]
    fn test_sentence() {
        let t = OpenAITokenizer::new();
        let count = t.count("The quick brown fox jumps over the lazy dog.");
        assert!(count > 5 && count < 20);
    }

    #[test]
    fn test_unicode() {
        let t = OpenAITokenizer::new();
        let count = t.count("Hello, 世界! 🌍");
        assert!(count > 0);
    }

    #[test]
    fn test_code() {
        let t = OpenAITokenizer::new();
        let count = t.count("fn main() { println!(\"hello\"); }");
        assert!(count > 0);
    }

    #[test]
    fn test_whitespace() {
        let t = OpenAITokenizer::new();
        let count = t.count("   \n\t   ");
        assert!(count > 0);  // Whitespace still takes tokens
    }

    #[test]
    fn test_consistency() {
        let t = OpenAITokenizer::new();
        let text = "This is a test of consistency.";
        assert_eq!(t.count(text), t.count(text));
    }

    #[test]
    fn test_long_text() {
        let t = OpenAITokenizer::new();
        let text = "word ".repeat(1000);
        let count = t.count(&text);
        assert!(count > 500 && count < 2000);
    }

    #[test]
    fn test_special_characters() {
        let t = OpenAITokenizer::new();
        let count = t.count("!@#$%^&*()_+-=[]{}|;':\",./<>?");
        assert!(count > 0);
    }

    #[test]
    fn test_newlines() {
        let t = OpenAITokenizer::new();
        let count = t.count("line1\nline2\nline3");
        assert!(count > 0);
    }
}
