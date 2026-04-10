//! Gemini CLI configurator.
//!
//! Copies embedded Gemini templates to `.gemini/`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::extract::{copy_embedded_dir, CopyOptions, GeminiTemplates};
use crate::templates::gemini as tmpl;

/// Configure Gemini CLI by copying all embedded templates to `.gemini/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".gemini");
    copy_embedded_dir::<GeminiTemplates>(&dest, &CopyOptions::default())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for cmd in tmpl::get_all_commands() {
        files.insert(
            format!(".gemini/commands/hc/{}.toml", cmd.name),
            cmd.content,
        );
    }

    files
}
