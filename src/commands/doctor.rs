//! Doctor command -- diagnose the development environment.
//!
//! Checks for required tools (git, python3), verifies that `.harness-cli/`
//! is properly initialized, and reports which AI platforms are configured.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::configurators::get_configured_platforms;
use crate::constants::paths::{constructed, dir_names};
use crate::constants::version::VERSION;
use crate::types::ai_tools::get_tool_config;

// =============================================================================
// DoctorOptions
// =============================================================================

/// Options for the `doctor` command.
pub struct DoctorOptions {}

// =============================================================================
// Check result tracking
// =============================================================================

/// Tracks pass / fail / warning counts across all checks.
struct CheckResults {
    issues: u32,
    warnings: u32,
}

impl CheckResults {
    fn new() -> Self {
        Self {
            issues: 0,
            warnings: 0,
        }
    }

    fn pass(&self, message: &str) {
        println!("    {} {}", "\u{2713}".green(), message);
    }

    fn fail(&mut self, message: &str) {
        self.issues += 1;
        println!("    {} {}", "\u{2717}".red(), message);
    }

    fn warn(&mut self, message: &str) {
        self.warnings += 1;
        println!("    {} {}", "\u{26A0}".yellow(), message);
    }
}

// =============================================================================
// Main doctor function
// =============================================================================

/// Run the `doctor` command.
///
/// Performs environment and project health checks, printing coloured results
/// to stdout.
pub fn doctor(_options: DoctorOptions) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let mut results = CheckResults::new();

    // Header
    println!();
    println!(
        "  {}",
        format!("Harness CLI Doctor (v{})", VERSION).bold()
    );
    println!("  {}", "\u{2500}".repeat(30).dimmed());

    // -- Section: Environment --
    println!();
    println!("  {}", "Environment".bold());
    let git_installed = check_git(&mut results);
    if git_installed {
        check_git_repo(&cwd, &mut results);
    }
    check_python(&mut results);

    // -- Section: Harness CLI --
    println!();
    println!("  {}", "Harness CLI".bold());
    check_harness_initialized(&cwd, &mut results);
    check_version_file(&cwd, &mut results);
    check_config_file(&cwd, &mut results);
    check_scripts_dir(&cwd, &mut results);
    check_workflow_guide(&cwd, &mut results);
    check_developer_identity(&cwd, &mut results);
    check_spec_dir(&cwd, &mut results);

    // -- Section: AI Platforms --
    println!();
    check_ai_platforms(&cwd, &mut results);

    // Footer
    println!("  {}", "\u{2500}".repeat(30).dimmed());

    let total_issues = results.issues + results.warnings;
    if total_issues == 0 {
        println!("  {}", "All checks passed!".green().bold());
    } else {
        let msg = format!(
            "{} issue(s) found",
            total_issues
        );
        println!("  {}", msg.red().bold());
        if results.issues > 0 {
            println!(
                "  {}",
                "  Run `harness-cli init` to fix missing files.".dimmed()
            );
        }
    }
    println!();

    Ok(())
}

// =============================================================================
// Environment checks
// =============================================================================

/// Check if `git` is available and print its version. Returns true if git is found.
fn check_git(results: &mut CheckResults) -> bool {
    match which::which("git") {
        Ok(_) => {
            let version = Command::new("git")
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    s.trim()
                        .strip_prefix("git version ")
                        .map(|v| v.to_string())
                })
                .unwrap_or_else(|| "unknown".to_string());
            results.pass(&format!("Git installed ({})", version));
            true
        }
        Err(_) => {
            results.fail("Git not installed");
            false
        }
    }
}

/// Check if the current directory is inside a git repository.
fn check_git_repo(cwd: &Path, results: &mut CheckResults) {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            results.pass("Inside git repository");
        }
        _ => {
            results.fail("Not inside a git repository");
        }
    }
}

/// Check if `python3` is available and version >= 3.6.
fn check_python(results: &mut CheckResults) {
    match which::which("python3") {
        Ok(_) => {
            let version_str = Command::new("python3")
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    // "Python 3.10.12" -> "3.10.12"
                    s.trim()
                        .strip_prefix("Python ")
                        .map(|v| v.to_string())
                });

            match version_str {
                Some(ver) => {
                    // Parse major.minor to check >= 3.6
                    let parts: Vec<&str> = ver.split('.').collect();
                    let major: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

                    if major > 3 || (major == 3 && minor >= 6) {
                        results.pass(&format!("Python {}", ver));
                    } else {
                        results.warn(&format!(
                            "Python {} (>= 3.6 recommended)",
                            ver
                        ));
                    }
                }
                None => {
                    results.warn("Python 3 installed (version unknown)");
                }
            }
        }
        Err(_) => {
            results.warn("Python 3 not installed (needed for harness-cli scripts)");
        }
    }
}

// =============================================================================
// Harness CLI checks
// =============================================================================

/// Check if `.harness-cli/` directory exists.
fn check_harness_initialized(cwd: &Path, results: &mut CheckResults) {
    let workflow_dir = cwd.join(constructed::WORKFLOW);
    if workflow_dir.exists() {
        results.pass("Initialized (.harness-cli/)");
    } else {
        results.fail("Not initialized (.harness-cli/ not found)");
    }
}

/// Check if `.harness-cli/.version` exists and matches the current CLI version.
fn check_version_file(cwd: &Path, results: &mut CheckResults) {
    let version_file = cwd.join(dir_names::WORKFLOW).join(".version");
    if version_file.exists() {
        match std::fs::read_to_string(&version_file) {
            Ok(content) => {
                let project_version = content.trim();
                if project_version == VERSION {
                    results.pass(&format!("Version: {} (up to date)", VERSION));
                } else {
                    results.warn(&format!(
                        "Version: {} (CLI is {})",
                        project_version, VERSION
                    ));
                }
            }
            Err(_) => {
                results.warn("Version file exists but unreadable");
            }
        }
    } else {
        results.fail("Version file missing (.version)");
    }
}

/// Check if `.harness-cli/config.yaml` exists.
fn check_config_file(cwd: &Path, results: &mut CheckResults) {
    let config_file = cwd.join(dir_names::WORKFLOW).join("config.yaml");
    if config_file.exists() {
        results.pass("Config: config.yaml");
    } else {
        results.fail("Config file missing (config.yaml)");
    }
}

/// Check if `.harness-cli/scripts/` exists.
fn check_scripts_dir(cwd: &Path, results: &mut CheckResults) {
    let scripts_dir = cwd.join(constructed::SCRIPTS);
    if scripts_dir.exists() {
        results.pass("Scripts: scripts/");
    } else {
        results.fail("Scripts directory missing (scripts/)");
    }
}

/// Check if `.harness-cli/workflow.md` exists.
fn check_workflow_guide(cwd: &Path, results: &mut CheckResults) {
    let workflow_file = cwd.join(constructed::WORKFLOW_GUIDE_FILE);
    if workflow_file.exists() {
        results.pass("Workflow guide: workflow.md");
    } else {
        results.fail("Workflow guide missing (workflow.md)");
    }
}

/// Check if `.harness-cli/.developer` exists and read the identity.
fn check_developer_identity(cwd: &Path, results: &mut CheckResults) {
    let developer_file = cwd.join(constructed::DEVELOPER_FILE);
    if developer_file.exists() {
        match std::fs::read_to_string(&developer_file) {
            Ok(content) => {
                // Parse key=value format; look for "name=..."
                let name = content
                    .lines()
                    .find_map(|line| line.strip_prefix("name="))
                    .unwrap_or("")
                    .trim();

                if name.is_empty() {
                    results.warn("Developer identity file is empty");
                } else {
                    results.pass(&format!("Developer identity: {}", name));
                }
            }
            Err(_) => {
                results.warn("Developer identity file exists but unreadable");
            }
        }
    } else {
        results.warn("Developer identity not set (.developer)");
    }
}

/// Check if `.harness-cli/spec/` exists.
fn check_spec_dir(cwd: &Path, results: &mut CheckResults) {
    let spec_dir = cwd.join(constructed::SPEC);
    if spec_dir.exists() {
        results.pass("Spec directory: spec/");
    } else {
        results.warn("Spec directory missing (spec/)");
    }
}

// =============================================================================
// AI Platform checks
// =============================================================================

/// Detect and list configured AI platforms.
fn check_ai_platforms(cwd: &Path, results: &mut CheckResults) {
    let platforms = get_configured_platforms(cwd);
    let count = platforms.len();

    println!(
        "  {}",
        format!("AI Platforms ({} configured)", count).bold()
    );

    if count == 0 {
        results.warn("No AI platforms configured");
        return;
    }

    // Sort platforms by name for consistent output
    let mut sorted: Vec<_> = platforms.iter().collect();
    sorted.sort_by_key(|tool| get_tool_config(**tool).name);

    for tool in sorted {
        let config = get_tool_config(*tool);
        results.pass(&format!("{} ({})", config.name, config.config_dir));
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_harness_dir(dir: &TempDir) {
        let workflow = dir.path().join(constructed::WORKFLOW);
        std::fs::create_dir_all(&workflow).unwrap();
        std::fs::write(workflow.join(".version"), VERSION).unwrap();
        std::fs::write(workflow.join("config.yaml"), "# config").unwrap();
        std::fs::create_dir_all(dir.path().join(constructed::SCRIPTS)).unwrap();
        std::fs::write(
            dir.path().join(constructed::WORKFLOW_GUIDE_FILE),
            "# guide",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join(constructed::SPEC)).unwrap();
    }

    #[test]
    fn test_check_harness_initialized_present() {
        let dir = TempDir::new().unwrap();
        setup_harness_dir(&dir);
        let mut results = CheckResults::new();
        check_harness_initialized(dir.path(), &mut results);
        assert_eq!(results.issues, 0);
    }

    #[test]
    fn test_check_harness_initialized_missing() {
        let dir = TempDir::new().unwrap();
        let mut results = CheckResults::new();
        check_harness_initialized(dir.path(), &mut results);
        assert_eq!(results.issues, 1);
    }

    #[test]
    fn test_check_version_file_matching() {
        let dir = TempDir::new().unwrap();
        setup_harness_dir(&dir);
        let mut results = CheckResults::new();
        check_version_file(dir.path(), &mut results);
        assert_eq!(results.issues, 0);
        assert_eq!(results.warnings, 0);
    }

    #[test]
    fn test_check_version_file_mismatch() {
        let dir = TempDir::new().unwrap();
        setup_harness_dir(&dir);
        let version_file = dir.path().join(dir_names::WORKFLOW).join(".version");
        std::fs::write(version_file, "0.0.1").unwrap();
        let mut results = CheckResults::new();
        check_version_file(dir.path(), &mut results);
        assert_eq!(results.warnings, 1);
    }

    #[test]
    fn test_check_version_file_missing() {
        let dir = TempDir::new().unwrap();
        // Create .harness-cli but no .version
        std::fs::create_dir_all(dir.path().join(dir_names::WORKFLOW)).unwrap();
        let mut results = CheckResults::new();
        check_version_file(dir.path(), &mut results);
        assert_eq!(results.issues, 1);
    }

    #[test]
    fn test_check_config_file_present() {
        let dir = TempDir::new().unwrap();
        setup_harness_dir(&dir);
        let mut results = CheckResults::new();
        check_config_file(dir.path(), &mut results);
        assert_eq!(results.issues, 0);
    }

    #[test]
    fn test_check_config_file_missing() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(dir_names::WORKFLOW)).unwrap();
        let mut results = CheckResults::new();
        check_config_file(dir.path(), &mut results);
        assert_eq!(results.issues, 1);
    }

    #[test]
    fn test_check_developer_identity_present() {
        let dir = TempDir::new().unwrap();
        setup_harness_dir(&dir);
        let dev_file = dir.path().join(constructed::DEVELOPER_FILE);
        std::fs::write(dev_file, "name=testuser\ninitialized_at=2026-01-01").unwrap();
        let mut results = CheckResults::new();
        check_developer_identity(dir.path(), &mut results);
        assert_eq!(results.issues, 0);
        assert_eq!(results.warnings, 0);
    }

    #[test]
    fn test_check_developer_identity_missing() {
        let dir = TempDir::new().unwrap();
        setup_harness_dir(&dir);
        let mut results = CheckResults::new();
        check_developer_identity(dir.path(), &mut results);
        assert_eq!(results.warnings, 1);
    }

    #[test]
    fn test_check_results_counters() {
        let mut results = CheckResults::new();
        assert_eq!(results.issues, 0);
        assert_eq!(results.warnings, 0);
        results.pass("test");
        assert_eq!(results.issues, 0);
        results.fail("test");
        assert_eq!(results.issues, 1);
        results.warn("test");
        assert_eq!(results.warnings, 1);
    }
}
