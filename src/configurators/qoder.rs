//! Qoder configurator.
//!
//! Copies embedded Qoder templates to `.qoder/`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::extract::{copy_embedded_dir, CopyOptions, QoderTemplates};
use crate::templates::qoder as tmpl;

/// Configure Qoder by copying all embedded templates to `.qoder/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".qoder");
    copy_embedded_dir::<QoderTemplates>(&dest, &CopyOptions::default())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for skill in tmpl::get_all_skills() {
        files.insert(
            format!(".qoder/skills/{}/SKILL.md", skill.name),
            skill.content,
        );
    }

    files
}
