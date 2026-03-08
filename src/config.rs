use crate::cli::{Args, TokenizerKind};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub budget: Budget,
    pub tokenizer: TokenizerKind,
    pub preserve_head: usize,
    pub preserve_tail: usize,
    pub stats: bool,
    pub strategies: StrategyConfig,
    pub thresholds: Thresholds,
    pub boilerplate_patterns: Vec<Regex>,
    pub preserve_patterns: Vec<Regex>,
    /// Preserve code blocks (never compress fenced code)
    pub preserve_code: bool,
    /// Use semantic chunking (respects headings, code blocks, paragraphs)
    pub semantic: bool,
}

#[derive(Debug, Clone)]
pub enum Budget {
    Absolute(usize),
    Percentage(f64),
    ModelBased { model: String, reserve: usize },
    NotSet,
}

#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub redundancy: bool,
    pub boilerplate: bool,
    pub extractive: bool,
    pub aggressive: bool,
    pub conservative: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Thresholds {
    #[serde(default = "default_redundancy_similarity")]
    pub redundancy_similarity: f64,
    #[serde(default = "default_importance_minimum")]
    pub importance_minimum: f64,
    #[serde(default = "default_boilerplate_confidence")]
    pub boilerplate_confidence: f64,
    #[serde(default = "default_position_decay")]
    pub position_decay: f64,
}

fn default_redundancy_similarity() -> f64 { 0.85 }
fn default_importance_minimum() -> f64 { 0.30 }
fn default_boilerplate_confidence() -> f64 { 0.90 }
fn default_position_decay() -> f64 { 0.95 }

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            redundancy_similarity: default_redundancy_similarity(),
            importance_minimum: default_importance_minimum(),
            boilerplate_confidence: default_boilerplate_confidence(),
            position_decay: default_position_decay(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid threshold {name}: {value} (must be 0.0-1.0)")]
    InvalidThreshold { name: String, value: f64 },
    #[error("invalid regex pattern: {0}")]
    InvalidPattern(#[from] regex::Error),
    #[error("invalid budget format: {0}")]
    InvalidBudget(String),
}

#[derive(Deserialize, Default)]
struct FileConfig {
    default_budget: Option<usize>,
    #[allow(dead_code)]
    tokenizer: Option<String>,
    #[allow(dead_code)]
    default_model: Option<String>,
    #[serde(default)]
    thresholds: Thresholds,
    #[serde(default)]
    boilerplate_patterns: std::collections::HashMap<String, String>,
    #[serde(default)]
    preserve_patterns: std::collections::HashMap<String, String>,
}

pub fn load(path: Option<&Path>) -> Result<Config, ConfigError> {
    let file_config: FileConfig = match path {
        Some(p) if p.exists() => {
            let content = std::fs::read_to_string(p)?;
            toml::from_str(&content)?
        }
        _ => FileConfig::default(),
    };
    
    // Validate thresholds
    validate_threshold("redundancy_similarity", file_config.thresholds.redundancy_similarity)?;
    validate_threshold("importance_minimum", file_config.thresholds.importance_minimum)?;
    validate_threshold("boilerplate_confidence", file_config.thresholds.boilerplate_confidence)?;
    validate_threshold("position_decay", file_config.thresholds.position_decay)?;
    
    // Compile boilerplate patterns
    let mut boilerplate_patterns = default_boilerplate_patterns()?;
    for (_, pattern) in file_config.boilerplate_patterns {
        boilerplate_patterns.push(Regex::new(&pattern)?);
    }
    
    // Compile preserve patterns
    let mut preserve_patterns = Vec::new();
    for (_, pattern) in file_config.preserve_patterns {
        preserve_patterns.push(Regex::new(&pattern)?);
    }
    
    Ok(Config {
        budget: match file_config.default_budget {
            Some(b) => Budget::Absolute(b),
            None => Budget::NotSet,
        },
        tokenizer: TokenizerKind::Openai,
        preserve_head: 0,
        preserve_tail: 0,
        stats: false,
        strategies: StrategyConfig {
            redundancy: true,
            boilerplate: true,
            extractive: true,
            aggressive: false,
            conservative: false,
        },
        thresholds: file_config.thresholds,
        boilerplate_patterns,
        preserve_patterns,
        preserve_code: false,
        semantic: false,
    })
}

fn validate_threshold(name: &str, value: f64) -> Result<(), ConfigError> {
    if !(0.0..=1.0).contains(&value) {
        return Err(ConfigError::InvalidThreshold {
            name: name.to_string(),
            value,
        });
    }
    Ok(())
}

fn default_boilerplate_patterns() -> Result<Vec<Regex>, ConfigError> {
    Ok(vec![
        Regex::new(r"(?i)^(MIT License|Apache License|GNU|BSD)")?,
        Regex::new(r"(?i)^(Copyright \(c\)|©)")?,
        Regex::new(r"(?i)^(Auto-generated|DO NOT EDIT|Generated by)")?,
        Regex::new(r"(?i)^(THE SOFTWARE IS PROVIDED|WITHOUT WARRANTY)")?,
        Regex::new(r"^[-=]{20,}$")?,
    ])
}

pub fn merge(mut config: Config, args: &Args) -> Result<Config, ConfigError> {
    // Budget
    if let Some(ref budget_str) = args.budget {
        config.budget = parse_budget(budget_str)?;
    } else if let Some(ref model) = args.model {
        config.budget = Budget::ModelBased {
            model: model.clone(),
            reserve: args.reserve,
        };
    }
    
    // Tokenizer
    config.tokenizer = args.tokenizer.clone();
    
    // Preserve
    config.preserve_head = args.preserve_head;
    config.preserve_tail = args.preserve_tail;
    
    // Stats
    config.stats = args.stats;
    
    // Strategies
    config.strategies.redundancy = !args.no_redundancy;
    config.strategies.boilerplate = !args.no_boilerplate;
    config.strategies.extractive = !args.no_extractive;
    config.strategies.aggressive = args.aggressive;
    config.strategies.conservative = args.conservative;
    
    // Preserve code blocks
    config.preserve_code = args.preserve_code;
    
    // Semantic chunking
    config.semantic = args.semantic;
    
    Ok(config)
}

fn parse_budget(s: &str) -> Result<Budget, ConfigError> {
    if let Some(pct_str) = s.strip_suffix('%') {
        let pct: f64 = pct_str.parse()
            .map_err(|_| ConfigError::InvalidBudget(s.to_string()))?;
        if !(0.0..=100.0).contains(&pct) || pct == 0.0 {
            return Err(ConfigError::InvalidBudget(format!("{}% out of range", pct)));
        }
        Ok(Budget::Percentage(pct / 100.0))
    } else {
        let n: usize = s.parse()
            .map_err(|_| ConfigError::InvalidBudget(s.to_string()))?;
        if n == 0 {
            return Err(ConfigError::InvalidBudget("budget cannot be 0".to_string()));
        }
        Ok(Budget::Absolute(n))
    }
}

#[allow(dead_code)]
pub fn default_config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("context-surgeon").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_absolute_budget() {
        let budget = parse_budget("8000").unwrap();
        assert!(matches!(budget, Budget::Absolute(8000)));
    }

    #[test]
    fn test_parse_percentage_budget() {
        let budget = parse_budget("25%").unwrap();
        if let Budget::Percentage(p) = budget {
            assert!((p - 0.25).abs() < 0.001);
        } else {
            panic!("expected percentage");
        }
    }

    #[test]
    fn test_parse_invalid_budget() {
        assert!(parse_budget("abc").is_err());
        assert!(parse_budget("0").is_err());
        assert!(parse_budget("150%").is_err());
    }

    #[test]
    fn test_default_thresholds() {
        let t = Thresholds::default();
        assert!((t.redundancy_similarity - 0.85).abs() < 0.001);
        assert!((t.importance_minimum - 0.30).abs() < 0.001);
    }

    #[test]
    fn test_validate_thresholds() {
        assert!(validate_threshold("test", 0.5).is_ok());
        assert!(validate_threshold("test", 1.5).is_err());
        assert!(validate_threshold("test", -0.1).is_err());
    }

    #[test]
    fn test_default_boilerplate_patterns() {
        let patterns = default_boilerplate_patterns().unwrap();
        assert!(patterns.len() >= 5);
        assert!(patterns[0].is_match("MIT License"));
    }
}
