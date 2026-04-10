//! Codex templates.
//!
//! Codex has:
//! - `skills/`       -- shared skills (written to `.agents/skills/`)
//! - `codex-skills/` -- Codex-specific skills (written to `.codex/skills/`)
//! - `agents/`       -- custom agents (`.toml` files)
//! - `hooks/`        -- hook scripts (`.py`)
//! - `hooks.json`    -- hooks config
//! - `config.toml`   -- project-scoped config

use super::extract::{get_embedded_file, list_files, CodexTemplates};

/// A skill template (directory name + SKILL.md content).
#[derive(Debug, Clone)]
pub struct SkillTemplate {
    pub name: String,
    pub content: String,
}

/// An agent template (name without extension + content).
#[derive(Debug, Clone)]
pub struct AgentTemplate {
    pub name: String,
    pub content: String,
}

/// A hook template (filename + content).
#[derive(Debug, Clone)]
pub struct HookTemplate {
    pub name: String,
    pub content: String,
}

/// A config template (target path + content).
#[derive(Debug, Clone)]
pub struct ConfigTemplate {
    pub target_path: String,
    pub content: String,
}

/// Get shared skills from `skills/<name>/SKILL.md` (installed to `.agents/skills/`).
pub fn get_all_skills() -> Vec<SkillTemplate> {
    let mut skills = Vec::new();
    for path in list_files::<CodexTemplates>() {
        if path.starts_with("skills/") && path.ends_with("/SKILL.md") {
            if let Some(content) = get_embedded_file::<CodexTemplates>(&path) {
                // Extract skill name: "skills/start/SKILL.md" -> "start"
                let name = path
                    .strip_prefix("skills/")
                    .unwrap()
                    .strip_suffix("/SKILL.md")
                    .unwrap()
                    .to_string();
                skills.push(SkillTemplate { name, content });
            }
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Get Codex-specific skills from `codex-skills/<name>/SKILL.md`.
pub fn get_all_codex_skills() -> Vec<SkillTemplate> {
    let mut skills = Vec::new();
    for path in list_files::<CodexTemplates>() {
        if path.starts_with("codex-skills/") && path.ends_with("/SKILL.md") {
            if let Some(content) = get_embedded_file::<CodexTemplates>(&path) {
                let name = path
                    .strip_prefix("codex-skills/")
                    .unwrap()
                    .strip_suffix("/SKILL.md")
                    .unwrap()
                    .to_string();
                skills.push(SkillTemplate { name, content });
            }
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Get all agent templates from `agents/*.toml`.
pub fn get_all_agents() -> Vec<AgentTemplate> {
    let mut agents = Vec::new();
    for path in list_files::<CodexTemplates>() {
        if path.starts_with("agents/") && path.ends_with(".toml") {
            if let Some(content) = get_embedded_file::<CodexTemplates>(&path) {
                let name = path
                    .strip_prefix("agents/")
                    .unwrap()
                    .strip_suffix(".toml")
                    .unwrap()
                    .to_string();
                agents.push(AgentTemplate { name, content });
            }
        }
    }
    agents
}

/// Get all hook scripts from `hooks/*.py`.
pub fn get_all_hooks() -> Vec<HookTemplate> {
    let mut hooks = Vec::new();
    for path in list_files::<CodexTemplates>() {
        if path.starts_with("hooks/") && path.ends_with(".py") {
            if let Some(content) = get_embedded_file::<CodexTemplates>(&path) {
                let name = path.strip_prefix("hooks/").unwrap().to_string();
                hooks.push(HookTemplate { name, content });
            }
        }
    }
    hooks
}

/// Get hooks.json config content.
pub fn get_hooks_config() -> String {
    get_embedded_file::<CodexTemplates>("hooks.json").unwrap_or_default()
}

/// Get config.toml template.
pub fn get_config_template() -> ConfigTemplate {
    let content =
        get_embedded_file::<CodexTemplates>("config.toml").unwrap_or_default();
    ConfigTemplate {
        target_path: "config.toml".to_string(),
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_skills_non_empty() {
        let skills = get_all_skills();
        assert!(!skills.is_empty(), "Codex shared skills should be non-empty");
    }

    #[test]
    fn test_get_all_codex_skills_non_empty() {
        let skills = get_all_codex_skills();
        assert!(
            !skills.is_empty(),
            "Codex platform-specific skills should be non-empty"
        );
    }

    #[test]
    fn test_get_all_agents_non_empty() {
        let agents = get_all_agents();
        assert!(!agents.is_empty(), "Codex agents should be non-empty");
    }

    #[test]
    fn test_get_all_hooks_non_empty() {
        let hooks = get_all_hooks();
        assert!(!hooks.is_empty(), "Codex hooks should be non-empty");
    }

    #[test]
    fn test_hooks_config_valid_json() {
        let config = get_hooks_config();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&config);
        assert!(
            parsed.is_ok(),
            "Codex hooks.json should be valid JSON, got error: {:?}",
            parsed.err()
        );
    }

    #[test]
    fn test_hooks_config_contains_python_cmd() {
        let config = get_hooks_config();
        assert!(
            config.contains("{{PYTHON_CMD}}"),
            "Codex hooks config should contain {{{{PYTHON_CMD}}}} placeholder"
        );
    }

    #[test]
    fn test_config_template_non_empty() {
        let config = get_config_template();
        assert!(
            !config.content.is_empty(),
            "Codex config.toml template should have content"
        );
    }
}
