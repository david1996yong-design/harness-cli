//! CodeBuddy configurator.
//!
//! Copies embedded CodeBuddy templates to `.codebuddy/`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::codebuddy as tmpl;
use crate::templates::extract::{copy_embedded_dir, CodeBuddyTemplates, CopyOptions};

/// Configure CodeBuddy by copying all embedded templates to `.codebuddy/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".codebuddy");
    copy_embedded_dir::<CodeBuddyTemplates>(&dest, &CopyOptions::default())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for cmd in tmpl::get_all_commands() {
        files.insert(
            format!(".codebuddy/commands/hc/{}.md", cmd.name),
            cmd.content,
        );
    }

    files
}
