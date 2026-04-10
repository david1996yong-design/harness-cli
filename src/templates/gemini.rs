//! Gemini CLI templates.
//!
//! Gemini CLI uses TOML format commands under `commands/hc/*.toml`.

use super::extract::{get_embedded_file, list_files, GeminiTemplates};

/// A command template (name without extension + content).
#[derive(Debug, Clone)]
pub struct CommandTemplate {
    pub name: String,
    pub content: String,
}

/// Get all command templates from `commands/hc/*.toml`.
pub fn get_all_commands() -> Vec<CommandTemplate> {
    let mut commands = Vec::new();
    for path in list_files::<GeminiTemplates>() {
        if path.starts_with("commands/hc/") && path.ends_with(".toml") {
            if let Some(content) = get_embedded_file::<GeminiTemplates>(&path) {
                let name = path
                    .strip_prefix("commands/hc/")
                    .unwrap()
                    .strip_suffix(".toml")
                    .unwrap()
                    .to_string();
                commands.push(CommandTemplate { name, content });
            }
        }
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_commands_non_empty() {
        let commands = get_all_commands();
        assert!(!commands.is_empty(), "Gemini commands should be non-empty");
    }

    #[test]
    fn test_command_names_no_extension() {
        for cmd in get_all_commands() {
            assert!(
                !cmd.name.ends_with(".toml"),
                "Command name '{}' should not include .toml extension",
                cmd.name
            );
        }
    }

    #[test]
    fn test_command_content_non_empty() {
        for cmd in get_all_commands() {
            assert!(
                !cmd.content.is_empty(),
                "Command '{}' should have non-empty content",
                cmd.name
            );
        }
    }
}
