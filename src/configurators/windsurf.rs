//! Windsurf configurator.
//!
//! Writes workflow templates to `.windsurf/workflows/<name>.md`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::windsurf as tmpl;
use crate::utils::file_writer::{ensure_dir, write_file};

/// Configure Windsurf by writing workflow templates.
pub fn configure(cwd: &Path) -> Result<()> {
    let workflow_root = cwd.join(".windsurf").join("workflows");
    ensure_dir(&workflow_root)?;

    for workflow in tmpl::get_all_workflows() {
        write_file(
            &workflow_root.join(format!("{}.md", workflow.name)),
            &workflow.content,
            false,
        )?;
    }

    Ok(())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for workflow in tmpl::get_all_workflows() {
        files.insert(
            format!(".windsurf/workflows/{}.md", workflow.name),
            workflow.content,
        );
    }

    files
}
