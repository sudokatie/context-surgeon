use crate::compression::CompressionStats;
use std::io::{self, Write};

pub struct StatsOutput;

impl StatsOutput {
    pub fn write<W: Write>(stats: &CompressionStats, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "Compression Statistics")?;
        writeln!(writer, "======================")?;
        writeln!(writer)?;
        writeln!(writer, "Original tokens:    {:>8}", stats.original_tokens)?;
        writeln!(writer, "Compressed tokens:  {:>8}", stats.compressed_tokens)?;
        writeln!(writer, "Tokens saved:       {:>8}", stats.tokens_saved())?;
        writeln!(writer, "Compression ratio:  {:>7.1}%", stats.compression_ratio() * 100.0)?;
        writeln!(writer)?;
        writeln!(writer, "Breakdown:")?;
        writeln!(writer, "  Redundancy removed: {:>6}", stats.redundancy_removed)?;
        writeln!(writer, "  Boilerplate removed:{:>6}", stats.boilerplate_removed)?;
        writeln!(writer, "  Extractive removed: {:>6}", stats.extractive_removed)?;
        writeln!(writer)?;
        writeln!(writer, "Segments: {}/{} kept", stats.segments_kept, stats.segments_total)?;
        Ok(())
    }
    
    pub fn to_stderr(stats: &CompressionStats) -> io::Result<()> {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        Self::write(stats, &mut handle)
    }
    
    #[allow(dead_code)]
    pub fn to_json<W: Write>(stats: &CompressionStats, writer: &mut W) -> io::Result<()> {
        let json = format!(
            r#"{{"original_tokens":{},"compressed_tokens":{},"tokens_saved":{},"compression_ratio":{:.4},"redundancy_removed":{},"boilerplate_removed":{},"extractive_removed":{},"segments_kept":{},"segments_total":{}}}"#,
            stats.original_tokens,
            stats.compressed_tokens,
            stats.tokens_saved(),
            stats.compression_ratio(),
            stats.redundancy_removed,
            stats.boilerplate_removed,
            stats.extractive_removed,
            stats.segments_kept,
            stats.segments_total,
        );
        writer.write_all(json.as_bytes())?;
        writeln!(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats() -> CompressionStats {
        CompressionStats {
            original_tokens: 1000,
            compressed_tokens: 500,
            redundancy_removed: 100,
            boilerplate_removed: 150,
            extractive_removed: 250,
            segments_kept: 5,
            segments_total: 10,
        }
    }

    #[test]
    fn test_write_contains_key_info() {
        let stats = make_stats();
        let mut buffer = Vec::new();
        StatsOutput::write(&stats, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        
        assert!(output.contains("1000"));  // original
        assert!(output.contains("500"));   // compressed
        assert!(output.contains("5/10"));  // segments
    }

    #[test]
    fn test_write_shows_breakdown() {
        let stats = make_stats();
        let mut buffer = Vec::new();
        StatsOutput::write(&stats, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        
        assert!(output.contains("100"));  // redundancy
        assert!(output.contains("150"));  // boilerplate
        assert!(output.contains("250"));  // extractive
    }

    #[test]
    fn test_json_output() {
        let stats = make_stats();
        let mut buffer = Vec::new();
        StatsOutput::to_json(&stats, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        
        assert!(output.contains("\"original_tokens\":1000"));
        assert!(output.contains("\"compressed_tokens\":500"));
        assert!(output.contains("\"segments_kept\":5"));
    }

    #[test]
    fn test_compression_ratio_display() {
        let stats = make_stats();
        let mut buffer = Vec::new();
        StatsOutput::write(&stats, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        
        // 50% compression ratio
        assert!(output.contains("50.0%"));
    }

    #[test]
    fn test_empty_stats() {
        let stats = CompressionStats::default();
        let mut buffer = Vec::new();
        StatsOutput::write(&stats, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        
        assert!(output.contains("0"));
    }
}
