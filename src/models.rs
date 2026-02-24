use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub name: &'static str,
    pub context_window: usize,
}

const MODELS: &[ModelInfo] = &[
    ModelInfo { name: "gpt-4o", context_window: 128_000 },
    ModelInfo { name: "gpt-4-turbo", context_window: 128_000 },
    ModelInfo { name: "gpt-4", context_window: 8_192 },
    ModelInfo { name: "gpt-3.5-turbo", context_window: 16_385 },
    ModelInfo { name: "claude-3-opus", context_window: 200_000 },
    ModelInfo { name: "claude-3-sonnet", context_window: 200_000 },
    ModelInfo { name: "claude-3-haiku", context_window: 200_000 },
    ModelInfo { name: "claude-3.5-sonnet", context_window: 200_000 },
];

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("unknown model '{0}', supported: {}", list_models().join(", "))]
    UnknownModel(String),
    #[error("reserve ({reserve}) must be less than context window ({window})")]
    ReserveExceedsWindow { reserve: usize, window: usize },
}

pub fn get_model(name: &str) -> Option<ModelInfo> {
    let lower = name.to_lowercase();
    MODELS.iter().find(|m| m.name.to_lowercase() == lower).copied()
}

pub fn list_models() -> Vec<&'static str> {
    MODELS.iter().map(|m| m.name).collect()
}

pub fn calculate_budget(model: &str, reserve: usize) -> Result<usize, ModelError> {
    let info = get_model(model).ok_or_else(|| ModelError::UnknownModel(model.to_string()))?;
    
    if reserve >= info.context_window {
        return Err(ModelError::ReserveExceedsWindow {
            reserve,
            window: info.context_window,
        });
    }
    
    Ok(info.context_window - reserve)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_gpt4o() {
        let model = get_model("gpt-4o").unwrap();
        assert_eq!(model.context_window, 128_000);
    }

    #[test]
    fn test_get_claude() {
        let model = get_model("claude-3-opus").unwrap();
        assert_eq!(model.context_window, 200_000);
    }

    #[test]
    fn test_case_insensitive() {
        assert!(get_model("GPT-4O").is_some());
        assert!(get_model("Claude-3-Opus").is_some());
    }

    #[test]
    fn test_unknown_model() {
        assert!(get_model("gpt-5").is_none());
    }

    #[test]
    fn test_calculate_budget() {
        let budget = calculate_budget("gpt-4o", 4000).unwrap();
        assert_eq!(budget, 124_000);
    }

    #[test]
    fn test_reserve_exceeds_window() {
        let result = calculate_budget("gpt-4", 10000);
        assert!(matches!(result, Err(ModelError::ReserveExceedsWindow { .. })));
    }

    #[test]
    fn test_list_models() {
        let models = list_models();
        assert!(models.len() >= 7);
        assert!(models.contains(&"gpt-4o"));
    }
}
