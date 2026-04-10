//! Kilo CLI configurator.
//!
//! Copies embedded Kilo templates to `.kilocode/`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::extract::{copy_embedded_dir, CopyOptions, KiloTemplates};
use crate::templates::kilo as tmpl;

/// Configure Kilo CLI by copying all embedded templates to `.kilocode/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".kilocode");
    copy_embedded_dir::<KiloTemplates>(&dest, &CopyOptions::default())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for wf in tmpl::get_all_workflows() {
        files.insert(
            format!(".kilocode/workflows/{}.md", wf.name),
            wf.content,
        );
    }

    files
}
