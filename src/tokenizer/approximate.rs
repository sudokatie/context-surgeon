use super::Tokenizer;

pub struct ApproximateTokenizer;

impl Tokenizer for ApproximateTokenizer {
    fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        
        let words = text.split_whitespace().count();
        let estimate = ((words as f64) * 1.3).ceil() as usize;
        
        estimate.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let t = ApproximateTokenizer;
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn test_single_word() {
        let t = ApproximateTokenizer;
        assert_eq!(t.count("hello"), 2);  // 1 * 1.3 = 1.3 -> 2
    }

    #[test]
    fn test_ten_words() {
        let t = ApproximateTokenizer;
        let count = t.count("one two three four five six seven eight nine ten");
        assert_eq!(count, 13);  // 10 * 1.3 = 13
    }

    #[test]
    fn test_whitespace_only() {
        let t = ApproximateTokenizer;
        assert_eq!(t.count("   \n\t   "), 1);  // minimum 1
    }

    #[test]
    fn test_consistency() {
        let t = ApproximateTokenizer;
        let text = "consistent test";
        assert_eq!(t.count(text), t.count(text));
    }
}
