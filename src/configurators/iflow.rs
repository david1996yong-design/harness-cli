//! iFlow CLI configurator.
//!
//! Copies embedded iFlow templates to `.iflow/`, resolving `{{PYTHON_CMD}}`
//! in `settings.json`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::extract::{copy_embedded_dir, CopyOptions, IflowTemplates};
use crate::templates::iflow as tmpl;

use super::shared::resolve_placeholders;

/// Configure iFlow CLI by copying all embedded templates to `.iflow/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".iflow");
    copy_embedded_dir::<IflowTemplates>(
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
            format!(".iflow/commands/hc/{}.md", cmd.name),
            cmd.content,
        );
    }

    for agent in tmpl::get_all_agents() {
        files.insert(
            format!(".iflow/agents/{}.md", agent.name),
            agent.content,
        );
    }

    for hook in tmpl::get_all_hooks() {
        files.insert(format!(".iflow/{}", hook.target_path), hook.content);
    }

    let settings = tmpl::get_settings_template();
    files.insert(
        format!(".iflow/{}", settings.target_path),
        resolve_placeholders(&settings.content),
    );

    files
}
