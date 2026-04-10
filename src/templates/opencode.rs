//! OpenCode templates.
//!
//! OpenCode uses commands and agents, similar to Claude. It also has a
//! `package.json` for its plugin system.

use super::extract::{get_embedded_file, list_files, OpenCodeTemplates};

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

/// Get all command templates from `commands/hc/*.md`.
pub fn get_all_commands() -> Vec<CommandTemplate> {
    let mut commands = Vec::new();
    for path in list_files::<OpenCodeTemplates>() {
        if path.starts_with("commands/hc/") && path.ends_with(".md") {
            if let Some(content) = get_embedded_file::<OpenCodeTemplates>(&path) {
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
    for path in list_files::<OpenCodeTemplates>() {
        if path.starts_with("agents/") && path.ends_with(".md") {
            if let Some(content) = get_embedded_file::<OpenCodeTemplates>(&path) {
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
