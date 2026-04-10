//! Cursor templates.
//!
//! Cursor uses a flat command directory with `hc-` prefix naming (no
//! subdirectory namespacing).

use super::extract::{get_embedded_file, list_files, CursorTemplates};

/// A command template (name without extension + content).
#[derive(Debug, Clone)]
pub struct CommandTemplate {
    pub name: String,
    pub content: String,
}

/// Get all command templates from `commands/*.md`.
///
/// Cursor uses `hc-start.md` style naming instead of `commands/hc/start.md`.
pub fn get_all_commands() -> Vec<CommandTemplate> {
    let mut commands = Vec::new();
    for path in list_files::<CursorTemplates>() {
        if path.starts_with("commands/") && path.ends_with(".md") {
            if let Some(content) = get_embedded_file::<CursorTemplates>(&path) {
                let name = path
                    .strip_prefix("commands/")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_commands_non_empty() {
        let commands = get_all_commands();
        assert!(!commands.is_empty(), "Cursor commands should be non-empty");
    }

    #[test]
    fn test_command_names_no_extension() {
        for cmd in get_all_commands() {
            assert!(
                !cmd.name.ends_with(".md"),
                "Command name '{}' should not include .md extension",
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
