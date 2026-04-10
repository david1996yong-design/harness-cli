//! Kiro Code configurator.
//!
//! Writes skill templates to `.kiro/skills/<name>/SKILL.md`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::kiro as tmpl;
use crate::utils::file_writer::{ensure_dir, write_file};

/// Configure Kiro Code by writing skill templates.
pub fn configure(cwd: &Path) -> Result<()> {
    let skills_root = cwd.join(".kiro").join("skills");
    ensure_dir(&skills_root)?;

    for skill in tmpl::get_all_skills() {
        let skill_dir = skills_root.join(&skill.name);
        ensure_dir(&skill_dir)?;
        write_file(&skill_dir.join("SKILL.md"), &skill.content, false)?;
    }

    Ok(())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for skill in tmpl::get_all_skills() {
        files.insert(
            format!(".kiro/skills/{}/SKILL.md", skill.name),
            skill.content,
        );
    }

    files
}
