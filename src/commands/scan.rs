//! Scan command -- create KB directory structure with templates.
//!
//! Creates `.harness-cli/kb/prd/` and `.harness-cli/kb/tech/` directories
//! with index and module-template files.

use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::constants::paths::constructed;
use crate::templates::markdown;
use crate::utils::file_writer::{ensure_dir, write_file};

// =============================================================================
// ScanOptions
// =============================================================================

/// Options for the `scan` command.
pub struct ScanOptions {
    /// Overwrite existing files without asking.
    pub force: bool,
}

// =============================================================================
// Main scan function
// =============================================================================

/// Run the `scan` command.
///
/// Creates the KB directory structure:
/// - `.harness-cli/kb/prd/` with `index.md` and `_module-template.md`
/// - `.harness-cli/kb/tech/` with `index.md` and `_module-template.md`
pub fn scan(options: ScanOptions) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Check that .harness-cli exists
    let workflow_dir = cwd.join(constructed::WORKFLOW);
    if !workflow_dir.exists() {
        println!(
            "{}",
            "Error: .harness-cli/ not found. Run `harness-cli init` first.".red()
        );
        return Ok(());
    }

    if options.force {
        crate::utils::file_writer::set_write_mode(crate::utils::file_writer::WriteMode::Force);
    }

    println!("{}", "\n  Creating KB directory structure...\n".blue());

    // Create kb/prd/
    create_kb_prd(&cwd)?;

    // Create kb/tech/
    create_kb_tech(&cwd)?;

    println!("{}", "\n  KB directory structure created!".green());
    println!(
        "{}",
        "  Next steps:".dimmed()
    );
    println!(
        "{}",
        "    - Run /hc:scan-kb to generate product knowledge (kb/prd/)".dimmed()
    );
    println!(
        "{}",
        "    - Run /hc:scan-kb-tech to generate architecture knowledge (kb/tech/)".dimmed()
    );

    Ok(())
}

// =============================================================================
// KB PRD creation
// =============================================================================

fn create_kb_prd(cwd: &Path) -> Result<()> {
    let kb_prd_dir = cwd.join(constructed::KB_PRD);
    ensure_dir(&kb_prd_dir)?;

    let index_path = kb_prd_dir.join("index.md");
    let template_path = kb_prd_dir.join("_module-template.md");

    let index_content = markdown::kb_prd_index_content();
    let template_content = markdown::kb_prd_module_template_content();

    let index_written = write_file(&index_path, index_content, false)?;
    if index_written {
        println!("{}", "    Created kb/prd/index.md".blue());
    }

    let template_written = write_file(&template_path, template_content, false)?;
    if template_written {
        println!("{}", "    Created kb/prd/_module-template.md".blue());
    }

    Ok(())
}

// =============================================================================
// KB Tech creation
// =============================================================================

fn create_kb_tech(cwd: &Path) -> Result<()> {
    let kb_tech_dir = cwd.join(constructed::KB_TECH);
    ensure_dir(&kb_tech_dir)?;

    let index_path = kb_tech_dir.join("index.md");
    let template_path = kb_tech_dir.join("_module-template.md");

    let index_content = markdown::kb_tech_index_content();
    let template_content = markdown::kb_tech_module_template_content();

    let index_written = write_file(&index_path, index_content, false)?;
    if index_written {
        println!("{}", "    Created kb/tech/index.md".blue());
    }

    let template_written = write_file(&template_path, template_content, false)?;
    if template_written {
        println!("{}", "    Created kb/tech/_module-template.md".blue());
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        // Create .harness-cli directory to simulate an initialized project
        fs::create_dir_all(dir.path().join(constructed::WORKFLOW)).unwrap();
        dir
    }

    #[test]
    fn test_create_kb_prd() {
        let dir = setup_project_dir();
        create_kb_prd(dir.path()).unwrap();

        let index_path = dir.path().join(constructed::KB_PRD).join("index.md");
        let template_path = dir
            .path()
            .join(constructed::KB_PRD)
            .join("_module-template.md");

        assert!(index_path.exists(), "kb/prd/index.md should exist");
        assert!(
            template_path.exists(),
            "kb/prd/_module-template.md should exist"
        );

        let index_content = fs::read_to_string(&index_path).unwrap();
        assert!(
            !index_content.is_empty(),
            "kb/prd/index.md should be non-empty"
        );
        assert!(
            index_content.contains("kb/prd/"),
            "kb/prd/index.md should reference kb/prd/"
        );
    }

    #[test]
    fn test_create_kb_tech() {
        let dir = setup_project_dir();
        create_kb_tech(dir.path()).unwrap();

        let index_path = dir.path().join(constructed::KB_TECH).join("index.md");
        let template_path = dir
            .path()
            .join(constructed::KB_TECH)
            .join("_module-template.md");

        assert!(index_path.exists(), "kb/tech/index.md should exist");
        assert!(
            template_path.exists(),
            "kb/tech/_module-template.md should exist"
        );

        let index_content = fs::read_to_string(&index_path).unwrap();
        assert!(
            !index_content.is_empty(),
            "kb/tech/index.md should be non-empty"
        );
        assert!(
            index_content.contains("kb/tech/"),
            "kb/tech/index.md should reference kb/tech/"
        );
    }

    #[test]
    fn test_create_kb_tech_has_five_docs_in_index() {
        let dir = setup_project_dir();
        create_kb_tech(dir.path()).unwrap();

        let index_path = dir.path().join(constructed::KB_TECH).join("index.md");
        let content = fs::read_to_string(&index_path).unwrap();

        // Verify the 5 fixed documents are referenced in the index
        assert!(
            content.contains("overview.md"),
            "index should reference overview.md"
        );
        assert!(
            content.contains("component-map.md"),
            "index should reference component-map.md"
        );
        assert!(
            content.contains("data-models.md"),
            "index should reference data-models.md"
        );
        assert!(
            content.contains("decisions.md"),
            "index should reference decisions.md"
        );
        assert!(
            content.contains("cross-cutting.md"),
            "index should reference cross-cutting.md"
        );
    }

    #[test]
    fn test_create_kb_prd_and_tech_together() {
        let dir = setup_project_dir();
        create_kb_prd(dir.path()).unwrap();
        create_kb_tech(dir.path()).unwrap();

        // Both directories should exist
        assert!(dir.path().join(constructed::KB_PRD).exists());
        assert!(dir.path().join(constructed::KB_TECH).exists());

        // Both should have index.md and _module-template.md
        assert!(
            dir.path()
                .join(constructed::KB_PRD)
                .join("index.md")
                .exists()
        );
        assert!(
            dir.path()
                .join(constructed::KB_PRD)
                .join("_module-template.md")
                .exists()
        );
        assert!(
            dir.path()
                .join(constructed::KB_TECH)
                .join("index.md")
                .exists()
        );
        assert!(
            dir.path()
                .join(constructed::KB_TECH)
                .join("_module-template.md")
                .exists()
        );
    }
}
