//! iFlow CLI templates.
//!
//! Same structure as Claude: commands, agents, hooks, and settings.json.

use super::extract::{get_embedded_file, list_files, IflowTemplates};

/// A command template (name without extension + content).
#[derive(Debug, Clone)]
pub struct CommandTemplate {
    pub name: String,
    pub content: String,
}

/// An agent template (name without extension + content).
#[derive(Debug, Clone)]
pub struct AgentTemplate {
    pub name: String,
    pub content: String,
}

/// A hook template (target path relative to config dir + content).
#[derive(Debug, Clone)]
pub struct HookTemplate {
    pub target_path: String,
    pub content: String,
}

/// A settings template (target path + content).
#[derive(Debug, Clone)]
pub struct SettingsTemplate {
    pub target_path: String,
    pub content: String,
}

/// Get all command templates from `commands/hc/*.md`.
pub fn get_all_commands() -> Vec<CommandTemplate> {
    let mut commands = Vec::new();
    for path in list_files::<IflowTemplates>() {
        if path.starts_with("commands/hc/") && path.ends_with(".md") {
            if let Some(content) = get_embedded_file::<IflowTemplates>(&path) {
                let name = path
                    .strip_prefix("commands/hc/")
                    .unwrap()
                    .strip_suffix(".md")
                    .unwrap()
                    .to_string();
                commands.push(CommandTemplate { name, content });
            }
        }
    }
    commands
}

/// Get all agent templates from `agents/*.md`.
pub fn get_all_agents() -> Vec<AgentTemplate> {
    let mut agents = Vec::new();
    for path in list_files::<IflowTemplates>() {
        if path.starts_with("agents/") && path.ends_with(".md") {
            if let Some(content) = get_embedded_file::<IflowTemplates>(&path) {
                let name = path
                    .strip_prefix("agents/")
                    .unwrap()
                    .strip_suffix(".md")
                    .unwrap()
                    .to_string();
                agents.push(AgentTemplate { name, content });
            }
        }
    }
    agents
}

/// Get all hook templates from `hooks/*`.
pub fn get_all_hooks() -> Vec<HookTemplate> {
    let mut hooks = Vec::new();
    for path in list_files::<IflowTemplates>() {
        if path.starts_with("hooks/") {
            if let Some(content) = get_embedded_file::<IflowTemplates>(&path) {
                hooks.push(HookTemplate {
                    target_path: path,
                    content,
                });
            }
        }
    }
    hooks
}

/// Get the settings.json template.
pub fn get_settings_template() -> SettingsTemplate {
    let content =
        get_embedded_file::<IflowTemplates>("settings.json").unwrap_or_default();
    SettingsTemplate {
        target_path: "settings.json".to_string(),
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_commands_non_empty() {
        let commands = get_all_commands();
        assert!(!commands.is_empty(), "iFlow commands should be non-empty");
    }

    #[test]
    fn test_get_all_agents_non_empty() {
        let agents = get_all_agents();
        assert!(!agents.is_empty(), "iFlow agents should be non-empty");
    }

    #[test]
    fn test_get_all_hooks_non_empty() {
        let hooks = get_all_hooks();
        assert!(!hooks.is_empty(), "iFlow hooks should be non-empty");
    }

    #[test]
    fn test_settings_valid_json() {
        let settings = get_settings_template();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&settings.content);
        assert!(
            parsed.is_ok(),
            "iFlow settings template should be valid JSON, got error: {:?}",
            parsed.err()
        );
    }

    #[test]
    fn test_settings_contains_python_cmd() {
        let settings = get_settings_template();
        assert!(
            settings.content.contains("{{PYTHON_CMD}}"),
            "iFlow settings template should contain {{{{PYTHON_CMD}}}} placeholder"
        );
    }
}
