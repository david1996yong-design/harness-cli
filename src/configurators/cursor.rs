//! Cursor configurator.
//!
//! Copies embedded Cursor templates to `.cursor/`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::cursor as tmpl;
use crate::templates::extract::{copy_embedded_dir, CopyOptions, CursorTemplates};

/// Configure Cursor by copying all embedded templates to `.cursor/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".cursor");
    copy_embedded_dir::<CursorTemplates>(&dest, &CopyOptions::default())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for cmd in tmpl::get_all_commands() {
        files.insert(format!(".cursor/commands/{}.md", cmd.name), cmd.content);
    }

    files
}
