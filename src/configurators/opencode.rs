//! OpenCode configurator.
//!
//! Copies embedded OpenCode templates to `.opencode/`.

use std::path::Path;

use anyhow::Result;

use crate::templates::extract::{copy_embedded_dir, CopyOptions, OpenCodeTemplates};

/// Configure OpenCode by copying all embedded templates to `.opencode/`.
pub fn configure(cwd: &Path) -> Result<()> {
    let dest = cwd.join(".opencode");
    copy_embedded_dir::<OpenCodeTemplates>(&dest, &CopyOptions::default())
}
