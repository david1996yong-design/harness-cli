//! Claude Code configurator.
//!
//! Copies embedded Claude templates to `.claude/` in the user's project,
//! resolving `{{PYTHON_CMD}}` in `settings.json`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::claude as tmpl;
use crate::templates::extract::{copy_embedded_dir, ClaudeTemplates, CopyOptions};

use super::shared::resolve_placeholders;

/// Configure Claude Code by copying all embedded templates to `.claude/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".claude");
    copy_embedded_dir::<ClaudeTemplates>(
        &dest,
        &CopyOptions {
            resolve_placeholders: true,
            placeholder_filename: Some("settings.json".to_string()),
            ..Default::default()
        },
    )
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for cmd in tmpl::get_all_commands() {
        files.insert(
            format!(".claude/commands/hc/{}.md", cmd.name),
            cmd.content,
        );
    }

    for agent in tmpl::get_all_agents() {
        files.insert(
            format!(".claude/agents/{}.md", agent.name),
            agent.content,
        );
    }

    for hook in tmpl::get_all_hooks() {
        files.insert(format!(".claude/{}", hook.target_path), hook.content);
    }

    let settings = tmpl::get_settings_template();
    files.insert(
        format!(".claude/{}", settings.target_path),
        resolve_placeholders(&settings.content),
    );

    files
}
