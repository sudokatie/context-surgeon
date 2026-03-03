use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "context-surgeon")]
#[command(version)]
#[command(about = "Intelligently compress context to fit token limits")]
#[command(long_about = "Compress large text contexts to fit within token budgets while preserving important information. Uses multiple strategies: redundancy removal, boilerplate filtering, and extractive compression.")]
pub struct Args {
    /// Input file (reads from stdin if not provided)
    #[arg(value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Target token budget (number or percentage like "25%")
    #[arg(long, value_name = "TOKENS")]
    pub budget: Option<String>,

    /// Model name for context window calculation
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Tokens to reserve for output when using --model
    #[arg(long, value_name = "TOKENS", default_value = "0")]
    pub reserve: usize,

    /// Always keep first N tokens
    #[arg(long, value_name = "TOKENS", default_value = "0")]
    pub preserve_head: usize,

    /// Always keep last N tokens
    #[arg(long, value_name = "TOKENS", default_value = "0")]
    pub preserve_tail: usize,

    /// Print compression statistics to stderr
    #[arg(long)]
    pub stats: bool,

    /// Tokenizer to use
    #[arg(long, value_enum, default_value = "openai")]
    pub tokenizer: TokenizerKind,

    /// Apply aggressive compression (may impact coherence)
    #[arg(long)]
    pub aggressive: bool,

    /// Apply conservative compression (may not reach budget)
    #[arg(long)]
    pub conservative: bool,

    /// Disable redundancy removal strategy
    #[arg(long)]
    pub no_redundancy: bool,

    /// Disable boilerplate filtering strategy
    #[arg(long)]
    pub no_boilerplate: bool,

    /// Disable extractive compression strategy
    #[arg(long)]
    pub no_extractive: bool,

    /// Preserve code blocks (never compress fenced code)
    #[arg(long)]
    pub preserve_code: bool,

    /// Path to config file
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum TokenizerKind {
    #[default]
    Openai,
    Anthropic,
    Approximate,
}

pub fn parse() -> Args {
    Args::parse()
}

#[derive(Debug, thiserror::Error)]
pub enum ArgError {
    #[error("either --budget or (--model with --reserve) is required")]
    NoBudget,
    #[error("--aggressive and --conservative are mutually exclusive")]
    ConflictingModes,
    #[error("--reserve requires --model")]
    ReserveWithoutModel,
}

pub fn validate(args: &Args) -> Result<(), ArgError> {
    // Must have budget or model
    if args.budget.is_none() && args.model.is_none() {
        return Err(ArgError::NoBudget);
    }
    
    // Can't be both aggressive and conservative
    if args.aggressive && args.conservative {
        return Err(ArgError::ConflictingModes);
    }
    
    // Reserve requires model
    if args.reserve > 0 && args.model.is_none() {
        return Err(ArgError::ReserveWithoutModel);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_budget() {
        let args = Args::parse_from(["test", "--budget", "8000"]);
        assert_eq!(args.budget, Some("8000".to_string()));
    }

    #[test]
    fn test_parse_percentage() {
        let args = Args::parse_from(["test", "--budget", "25%"]);
        assert_eq!(args.budget, Some("25%".to_string()));
    }

    #[test]
    fn test_parse_model() {
        let args = Args::parse_from(["test", "--model", "gpt-4o", "--reserve", "4000"]);
        assert_eq!(args.model, Some("gpt-4o".to_string()));
        assert_eq!(args.reserve, 4000);
    }

    #[test]
    fn test_validate_no_budget() {
        let args = Args::parse_from(["test"]);
        assert!(validate(&args).is_err());
    }

    #[test]
    fn test_validate_conflicting_modes() {
        let args = Args::parse_from(["test", "--budget", "8000", "--aggressive", "--conservative"]);
        assert!(matches!(validate(&args), Err(ArgError::ConflictingModes)));
    }

    #[test]
    fn test_validate_reserve_without_model() {
        let args = Args::parse_from(["test", "--budget", "8000", "--reserve", "1000"]);
        assert!(matches!(validate(&args), Err(ArgError::ReserveWithoutModel)));
    }

    #[test]
    fn test_tokenizer_kinds() {
        let args = Args::parse_from(["test", "--budget", "8000", "--tokenizer", "anthropic"]);
        assert!(matches!(args.tokenizer, TokenizerKind::Anthropic));
    }

    #[test]
    fn test_strategy_flags() {
        let args = Args::parse_from(["test", "--budget", "8000", "--no-redundancy", "--no-boilerplate"]);
        assert!(args.no_redundancy);
        assert!(args.no_boilerplate);
        assert!(!args.no_extractive);
    }

    #[test]
    fn test_preserve_flags() {
        let args = Args::parse_from(["test", "--budget", "8000", "--preserve-head", "100", "--preserve-tail", "50"]);
        assert_eq!(args.preserve_head, 100);
        assert_eq!(args.preserve_tail, 50);
    }

    #[test]
    fn test_file_input() {
        let args = Args::parse_from(["test", "--budget", "8000", "input.txt"]);
        assert_eq!(args.input, Some(PathBuf::from("input.txt")));
    }

    #[test]
    fn test_stats_flag() {
        let args = Args::parse_from(["test", "--budget", "8000", "--stats"]);
        assert!(args.stats);
    }

    #[test]
    fn test_preserve_code_flag() {
        let args = Args::parse_from(["test", "--budget", "8000", "--preserve-code"]);
        assert!(args.preserve_code);
    }
}
