mod cli;
mod config;
mod models;
mod segmenter;
mod tokenizer;
mod analysis;
mod compression;
mod output;
mod progress;
mod coherence;

use config::Budget;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("input error: {0}")]
    Input(String),
    #[error("argument error: {0}")]
    Argument(String),
    #[error("budget error: {0}")]
    Budget(String),
    #[error("model error: {0}")]
    Model(#[from] models::ModelError),
    #[error("config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("compression error: {0}")]
    Compression(#[from] compression::CompressionError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Input(_) => 1,
            Error::Argument(_) => 2,
            Error::Budget(_) | Error::Model(_) => 3,
            Error::Config(_) | Error::Compression(_) | Error::Io(_) => 4,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(e.exit_code());
    }
}

fn run() -> Result<(), Error> {
    // Parse and validate CLI args
    let args = cli::parse();
    cli::validate(&args).map_err(|e| Error::Argument(e.to_string()))?;
    
    // Load and merge config
    let config_path = args.config.as_deref();
    let base_config = config::load(config_path)?;
    let config = config::merge(base_config, &args)?;
    
    // Read input
    let input = read_input(args.input.as_deref())?;
    if input.trim().is_empty() {
        return Err(Error::Input("empty input".into()));
    }
    
    // Create tokenizer
    let tokenizer = tokenizer::create(&config.tokenizer);
    
    // Get initial token count for progress tracking
    let initial_tokens = tokenizer.count(&input);
    
    // Initialize progress tracker (shows bar for >50k tokens)
    let progress = progress::ProgressTracker::new(initial_tokens);
    
    // Calculate budget
    let budget = resolve_budget(&config.budget, &input, tokenizer.as_ref())?;
    
    // Segment input
    progress.segmenting();
    let segments = if config.preserve_code {
        segmenter::segment_with_code_blocks(&input, segmenter::SegmentMode::Paragraph)
    } else {
        segmenter::segment(&input, segmenter::SegmentMode::Paragraph)
    };
    
    // Early exit if no segments
    if segments.is_empty() {
        progress.finish();
        print!("{}", input);
        if config.stats {
            let token_count = tokenizer.count(&input);
            output::stats::StatsOutput::to_stderr(&compression::CompressionStats {
                original_tokens: token_count,
                compressed_tokens: token_count,
                segments_kept: 0,
                segments_total: 0,
                ..Default::default()
            })?;
        }
        return Ok(());
    }
    
    // Analyze
    progress.analyzing_redundancy();
    let analysis = analysis::analyze(segments, tokenizer.as_ref(), &config);
    
    progress.analyzing_importance();
    
    // Check if compression needed
    if analysis.total_tokens <= budget {
        progress.finish();
        print!("{}", input);
        if config.stats {
            output::stats::StatsOutput::to_stderr(&compression::CompressionStats {
                original_tokens: analysis.total_tokens,
                compressed_tokens: analysis.total_tokens,
                segments_kept: analysis.segments.len(),
                segments_total: analysis.segments.len(),
                ..Default::default()
            })?;
        }
        return Ok(());
    }
    
    // Compress
    progress.compressing();
    let result = compression::compress(
        analysis.clone(),
        budget,
        config.preserve_head,
        config.preserve_tail,
        &config,
    )?;
    
    // Perform coherence check
    progress.reassembling();
    let segment_texts: Vec<String> = analysis.segments.iter()
        .map(|s| s.segment.text.clone())
        .collect();
    let kept_indices: Vec<usize> = (0..analysis.segments.len())
        .filter(|&i| {
            !analysis.segments[i].is_redundant && 
            !analysis.segments[i].is_boilerplate
        })
        .collect();
    
    let warnings = coherence::CoherenceChecker::check(&segment_texts, &kept_indices);
    
    progress.finish();
    
    // Output compressed text
    output::text::TextOutput::to_stdout(&result)?;
    
    // Output stats if requested
    if config.stats {
        output::stats::StatsOutput::to_stderr(&result.stats)?;
    }
    
    // Print coherence warnings (warnings only, exit 0)
    if !warnings.is_empty() {
        coherence::CoherenceChecker::print_warnings(&warnings);
    }
    
    Ok(())
}

fn read_input(path: Option<&Path>) -> Result<String, Error> {
    match path {
        Some(p) => {
            if !p.exists() {
                return Err(Error::Input(format!("file not found: {}", p.display())));
            }
            fs::read_to_string(p)
                .map_err(|e| Error::Input(format!("{}: {}", p.display(), e)))
        }
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| Error::Input(e.to_string()))?;
            Ok(buf)
        }
    }
}

fn resolve_budget(
    budget: &Budget,
    input: &str,
    tokenizer: &dyn tokenizer::Tokenizer,
) -> Result<usize, Error> {
    match budget {
        Budget::Absolute(n) => {
            if *n == 0 {
                return Err(Error::Budget("budget cannot be zero".into()));
            }
            Ok(*n)
        }
        Budget::Percentage(p) => {
            let total = tokenizer.count(input);
            let target = (total as f64 * p).ceil() as usize;
            if target == 0 {
                return Err(Error::Budget("percentage budget results in zero tokens".into()));
            }
            Ok(target)
        }
        Budget::ModelBased { model, reserve } => {
            models::calculate_budget(model, *reserve).map_err(Error::Model)
        }
        Budget::NotSet => Err(Error::Budget("no budget specified".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::ApproximateTokenizer;

    #[test]
    fn test_resolve_absolute_budget() {
        let budget = Budget::Absolute(1000);
        let tokenizer = ApproximateTokenizer;
        let result = resolve_budget(&budget, "test", &tokenizer).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn test_resolve_percentage_budget() {
        let budget = Budget::Percentage(0.5);
        let tokenizer = ApproximateTokenizer;
        let result = resolve_budget(&budget, "one two three four five six seven eight nine ten", &tokenizer).unwrap();
        assert!(result > 0);
    }

    #[test]
    fn test_resolve_model_budget() {
        let budget = Budget::ModelBased {
            model: "gpt-4o".to_string(),
            reserve: 4000,
        };
        let tokenizer = ApproximateTokenizer;
        let result = resolve_budget(&budget, "test", &tokenizer).unwrap();
        assert_eq!(result, 128_000 - 4000);
    }

    #[test]
    fn test_resolve_zero_budget_error() {
        let budget = Budget::Absolute(0);
        let tokenizer = ApproximateTokenizer;
        assert!(resolve_budget(&budget, "test", &tokenizer).is_err());
    }

    #[test]
    fn test_resolve_not_set_error() {
        let budget = Budget::NotSet;
        let tokenizer = ApproximateTokenizer;
        assert!(resolve_budget(&budget, "test", &tokenizer).is_err());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_input(Some(Path::new("/nonexistent/file.txt")));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(Error::Input("test".into()).exit_code(), 1);
        assert_eq!(Error::Argument("test".into()).exit_code(), 2);
        assert_eq!(Error::Budget("test".into()).exit_code(), 3);
        assert_eq!(Error::Io(io::Error::new(io::ErrorKind::Other, "test")).exit_code(), 4);
    }
}
