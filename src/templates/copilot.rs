//! GitHub Copilot templates.
//!
//! Copilot uses:
//! - `prompts/*.prompt.md` -- slash-command prompt files
//! - `hooks/*.py`          -- hook scripts
//! - `hooks.json`          -- hooks configuration

use super::extract::{get_embedded_file, list_files, CopilotTemplates};

/// A hook template (filename + content).
#[derive(Debug, Clone)]
pub struct HookTemplate {
    pub name: String,
    pub content: String,
}

/// A prompt template (name without `.prompt.md` suffix + content).
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub content: String,
}

/// Get all hook scripts from `hooks/*.py`.
pub fn get_all_hooks() -> Vec<HookTemplate> {
    let mut hooks = Vec::new();
    for path in list_files::<CopilotTemplates>() {
        if path.starts_with("hooks/") && path.ends_with(".py") {
            if let Some(content) = get_embedded_file::<CopilotTemplates>(&path) {
                let name = path.strip_prefix("hooks/").unwrap().to_string();
                hooks.push(HookTemplate { name, content });
            }
        }
    }
    hooks
}

/// Get hooks.json config content.
pub fn get_hooks_config() -> String {
    get_embedded_file::<CopilotTemplates>("hooks.json").unwrap_or_default()
}

/// Get all prompt templates from `prompts/*.prompt.md`.
pub fn get_all_prompts() -> Vec<PromptTemplate> {
    let mut prompts = Vec::new();
    for path in list_files::<CopilotTemplates>() {
        if path.starts_with("prompts/") && path.ends_with(".prompt.md") {
            if let Some(content) = get_embedded_file::<CopilotTemplates>(&path) {
                let name = path
                    .strip_prefix("prompts/")
                    .unwrap()
                    .strip_suffix(".prompt.md")
                    .unwrap()
                    .to_string();
                prompts.push(PromptTemplate { name, content });
            }
        }
    }
    prompts.sort_by(|a, b| a.name.cmp(&b.name));
    prompts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_hooks_non_empty() {
        let hooks = get_all_hooks();
        assert!(!hooks.is_empty(), "Copilot hooks should be non-empty");
    }

    #[test]
    fn test_hooks_config_valid_json() {
        let config = get_hooks_config();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&config);
        assert!(
            parsed.is_ok(),
            "Copilot hooks.json should be valid JSON, got error: {:?}",
            parsed.err()
        );
    }

    #[test]
    fn test_hooks_config_contains_python_cmd() {
        let config = get_hooks_config();
        assert!(
            config.contains("{{PYTHON_CMD}}"),
            "Copilot hooks config should contain {{{{PYTHON_CMD}}}} placeholder"
        );
    }

    #[test]
    fn test_get_all_prompts_non_empty() {
        let prompts = get_all_prompts();
        assert!(!prompts.is_empty(), "Copilot prompts should be non-empty");
    }

    #[test]
    fn test_prompt_content_non_empty() {
        for prompt in get_all_prompts() {
            assert!(
                !prompt.content.is_empty(),
                "Prompt '{}' should have non-empty content",
                prompt.name
            );
        }
    }
}
