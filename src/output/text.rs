use crate::compression::CompressionResult;
use std::io::{self, Write};

pub struct TextOutput;

impl TextOutput {
    pub fn write<W: Write>(result: &CompressionResult, writer: &mut W) -> io::Result<()> {
        writer.write_all(result.output.as_bytes())?;
        Ok(())
    }
    
    pub fn to_stdout(result: &CompressionResult) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        Self::write(result, &mut handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::CompressionStats;

    fn make_result(text: &str) -> CompressionResult {
        CompressionResult {
            output: text.to_string(),
            stats: CompressionStats::default(),
        }
    }

    #[test]
    fn test_write_to_buffer() {
        let result = make_result("Hello, world!");
        let mut buffer = Vec::new();
        TextOutput::write(&result, &mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), "Hello, world!");
    }

    #[test]
    fn test_write_empty() {
        let result = make_result("");
        let mut buffer = Vec::new();
        TextOutput::write(&result, &mut buffer).unwrap();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_write_unicode() {
        let result = make_result("Hello, 世界! 🌍");
        let mut buffer = Vec::new();
        TextOutput::write(&result, &mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), "Hello, 世界! 🌍");
    }

    #[test]
    fn test_write_multiline() {
        let result = make_result("Line 1\nLine 2\nLine 3");
        let mut buffer = Vec::new();
        TextOutput::write(&result, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 2"));
    }
}
