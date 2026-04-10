//! Embedded template assets and helpers for extracting them to disk.
//!
//! Each platform has its own `rust_embed::Embed` struct that includes only the
//! non-TypeScript, non-build-artifact files from the corresponding
//! `embedded/templates/<platform>/` directory.

use std::path::Path;

use anyhow::Result;
use rust_embed::Embed;

use crate::utils::file_writer::{ensure_dir, write_file};

// ---------------------------------------------------------------------------
// Embedded asset structs
// ---------------------------------------------------------------------------

#[derive(Embed)]
#[folder = "embedded/templates/claude/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct ClaudeTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/cursor/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct CursorTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/iflow/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct IflowTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/opencode/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
#[exclude = "node_modules/*"]
#[exclude = "bun.lock"]
#[exclude = ".gitignore"]
pub struct OpenCodeTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/codex/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct CodexTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/kilo/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
#[exclude = "node_modules/*"]
#[exclude = "bun.lock"]
#[exclude = ".gitignore"]
pub struct KiloTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/kiro/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct KiroTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/gemini/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
#[exclude = "node_modules/*"]
#[exclude = "bun.lock"]
#[exclude = ".gitignore"]
pub struct GeminiTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/windsurf/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct WindsurfTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/qoder/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
#[exclude = "node_modules/*"]
#[exclude = "bun.lock"]
#[exclude = ".gitignore"]
pub struct QoderTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/codebuddy/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct CodeBuddyTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/copilot/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct CopilotTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/harness-cli/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct HarnessCliTemplates;

#[derive(Embed)]
#[folder = "embedded/templates/markdown/"]
#[exclude = "*.ts"]
#[exclude = "*.js"]
#[exclude = "*.d.ts"]
#[exclude = "*.map"]
#[exclude = "__pycache__/*"]
pub struct MarkdownTemplates;

// ---------------------------------------------------------------------------
// Copy options
// ---------------------------------------------------------------------------

/// Options for copying embedded files to disk.
#[derive(Debug, Clone, Default)]
pub struct CopyOptions {
    /// Mark `.sh` and `.py` files as executable (Unix only).
    pub executable: bool,
    /// If set, resolve `{{PYTHON_CMD}}` placeholders in content.
    pub resolve_placeholders: bool,
    /// Only resolve placeholders in files matching this name (e.g. `settings.json`).
    pub placeholder_filename: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Retrieve a single embedded file's content as a UTF-8 string.
pub fn get_embedded_file<T: Embed>(path: &str) -> Option<String> {
    T::get(path).map(|f| String::from_utf8_lossy(&f.data).into_owned())
}

/// List all file paths inside an embedded asset.
pub fn list_files<T: Embed>() -> Vec<String> {
    T::iter().map(|f| f.to_string()).collect()
}

/// Copy all embedded files of type `T` into `dest`, creating subdirectories as
/// needed.
pub fn copy_embedded_dir<T: Embed>(dest: &Path, options: &CopyOptions) -> Result<()> {
    ensure_dir(dest)?;

    for file_path in T::iter() {
        let file_path_str = file_path.as_ref();

        let Some(asset) = T::get(file_path_str) else {
            continue;
        };

        let mut content = String::from_utf8_lossy(&asset.data).into_owned();

        // Optionally resolve placeholders
        if options.resolve_placeholders {
            let should_resolve = match &options.placeholder_filename {
                Some(name) => {
                    Path::new(file_path_str)
                        .file_name()
                        .map(|f| f.to_string_lossy() == name.as_str())
                        .unwrap_or(false)
                }
                None => true,
            };
            if should_resolve {
                content = crate::configurators::shared::resolve_placeholders(&content);
            }
        }

        let target = dest.join(file_path_str);

        // Ensure parent directory exists
        if let Some(parent) = target.parent() {
            ensure_dir(parent)?;
        }

        let is_executable = options.executable
            && (file_path_str.ends_with(".sh") || file_path_str.ends_with(".py"));

        write_file(&target, &content, is_executable)?;
    }

    Ok(())
}
