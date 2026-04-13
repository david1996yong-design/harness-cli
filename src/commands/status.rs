//! Status command -- show project status at a glance.
//!
//! Displays project info, developer identity, configured AI platforms,
//! active tasks, spec layers, knowledge base status, and git status.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::configurators::get_configured_platforms;
use crate::constants::paths::{constructed, dir_names};
use crate::constants::version::VERSION;
use crate::types::ai_tools::get_tool_config;
use crate::utils::project_detector::{detect_project_type, ProjectType};

// =============================================================================
// StatusOptions
// =============================================================================

/// Options for the `status` command.
pub struct StatusOptions {}

// =============================================================================
// Main status function
// =============================================================================

/// Run the `status` command.
///
/// Displays a summary of the current project including version, type,
/// developer identity, configured platforms, tasks, spec layers,
/// knowledge base, and git status.
pub fn status(_options: StatusOptions) -> Result<()> {
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

    println!();
    println!("  {}", "Harness CLI Status".bold());
    println!("  {}", "\u{2500}".repeat(30).dimmed());

    // -- Project section --
    print_project_section(&cwd)?;

    // -- AI Platforms section --
    print_platforms_section(&cwd);

    // -- Tasks section --
    print_tasks_section(&cwd)?;

    // -- Spec Layers section --
    print_spec_section(&cwd);

    // -- Knowledge Base section --
    print_kb_section(&cwd);

    // -- Git section --
    print_git_section(&cwd);

    println!();

    Ok(())
}

// =============================================================================
// Project section
// =============================================================================

fn print_project_section(cwd: &Path) -> Result<()> {
    println!();
    println!("  {}", "Project".bold());

    // Version
    println!("    {:<14}{}", "Version:".dimmed(), VERSION);

    // Project type
    let project_type = detect_project_type(cwd);
    let type_label = match project_type {
        ProjectType::Frontend => "Frontend",
        ProjectType::Backend => "Backend",
        ProjectType::Fullstack => "Fullstack",
        ProjectType::Unknown => "Unknown",
    };
    println!("    {:<14}{}", "Type:".dimmed(), type_label);

    // Developer identity
    let developer_file = cwd.join(constructed::DEVELOPER_FILE);
    if let Ok(content) = fs::read_to_string(&developer_file) {
        // Parse name=value format, extract only the name field
        let developer = content
            .lines()
            .find_map(|line| line.strip_prefix("name="))
            .unwrap_or("")
            .trim();
        if !developer.is_empty() {
            println!("    {:<14}{}", "Developer:".dimmed(), developer);
        }
    }

    Ok(())
}

// =============================================================================
// AI Platforms section
// =============================================================================

fn print_platforms_section(cwd: &Path) {
    let platforms = get_configured_platforms(cwd);

    if platforms.is_empty() {
        return;
    }

    println!();
    println!("  {}", "AI Platforms".bold());

    let mut names: Vec<&str> = platforms
        .iter()
        .map(|p| get_tool_config(*p).name)
        .collect();
    names.sort();

    // Print platforms in rows, multiple per line
    let line = names
        .iter()
        .map(|n| format!("{} {}", "\u{2022}".dimmed(), n))
        .collect::<Vec<_>>()
        .join("    ");
    println!("    {}", line);
}

// =============================================================================
// Tasks section
// =============================================================================

fn print_tasks_section(cwd: &Path) -> Result<()> {
    let tasks_dir = cwd.join(constructed::TASKS);

    // Read current task pointer
    let current_task_file = cwd.join(constructed::CURRENT_TASK_FILE);
    let current_task = fs::read_to_string(&current_task_file)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if !tasks_dir.exists() {
        return Ok(());
    }

    // List task directories (skip archive/)
    let mut tasks: Vec<(String, String, String)> = Vec::new(); // (dir_name, title, status)
    if let Ok(entries) = fs::read_dir(&tasks_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip archive directory
            if name == dir_names::ARCHIVE {
                continue;
            }

            // Only consider directories
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }

            // Try to read task.json
            let task_json_path = entry.path().join("task.json");
            let (title, task_status) = if let Ok(content) = fs::read_to_string(&task_json_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let title = json
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&name)
                        .to_string();
                    let status = json
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    (title, status)
                } else {
                    (name.clone(), "unknown".to_string())
                }
            } else {
                (name.clone(), "unknown".to_string())
            };

            tasks.push((name, title, task_status));
        }
    }

    // Sort tasks by directory name
    tasks.sort_by(|a, b| a.0.cmp(&b.0));

    println!();
    if tasks.is_empty() {
        println!("  {}", "Tasks (none)".bold());
    } else {
        println!(
            "  {}",
            format!("Tasks ({} active)", tasks.len()).bold()
        );
        for (dir_name, title, task_status) in &tasks {
            // current-task stores full relative path like ".harness-cli/tasks/04-13-xxx"
            // so extract the final component for comparison
            let is_current = current_task
                .as_deref()
                .map(|ct| {
                    ct.rsplit('/').next().unwrap_or(ct) == dir_name.as_str()
                })
                .unwrap_or(false);
            let marker = if is_current {
                "\u{2192}".green().to_string()
            } else {
                " ".to_string()
            };
            // Show title if it differs from dir_name, otherwise just show dir_name
            let display = if title != dir_name {
                format!("{} - {}", dir_name, title)
            } else {
                dir_name.clone()
            };
            println!(
                "    {} {} ({})",
                marker,
                display,
                task_status.dimmed()
            );
        }
    }

    Ok(())
}

// =============================================================================
// Spec Layers section
// =============================================================================

fn print_spec_section(cwd: &Path) {
    let spec_dir = cwd.join(constructed::SPEC);

    if !spec_dir.exists() {
        return;
    }

    let mut layers: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&spec_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                layers.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    if layers.is_empty() {
        return;
    }

    layers.sort();

    println!();
    println!("  {}", "Spec Layers".bold());
    let line = layers
        .iter()
        .map(|l| format!("{} {}", "\u{2022}".dimmed(), l))
        .collect::<Vec<_>>()
        .join("    ");
    println!("    {}", line);
}

// =============================================================================
// Knowledge Base section
// =============================================================================

fn print_kb_section(cwd: &Path) {
    let kb_prd_dir = cwd.join(constructed::KB_PRD);
    let kb_tech_dir = cwd.join(constructed::KB_TECH);

    let prd_exists = kb_prd_dir.exists();
    let tech_exists = kb_tech_dir.exists();

    if !prd_exists && !tech_exists {
        return;
    }

    println!();
    println!("  {}", "Knowledge Base".bold());

    let mut items: Vec<String> = Vec::new();

    if prd_exists {
        let count = count_files_in_dir(&kb_prd_dir);
        items.push(format!(
            "{} kb/prd ({} files)",
            "\u{2022}".dimmed(),
            count
        ));
    }

    if tech_exists {
        let count = count_files_in_dir(&kb_tech_dir);
        items.push(format!(
            "{} kb/tech ({} files)",
            "\u{2022}".dimmed(),
            count
        ));
    }

    let line = items.join("    ");
    println!("    {}", line);
}

/// Count the number of files (not directories) in a directory.
fn count_files_in_dir(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
}

// =============================================================================
// Git section
// =============================================================================

fn print_git_section(cwd: &Path) {
    println!();
    println!("  {}", "Git".bold());

    // Current branch
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // git branch --show-current returns empty on detached HEAD
    let branch = if branch.is_empty() {
        // Try to get short SHA for detached HEAD
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(cwd)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    Some(format!("detached at {}", sha))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "detached HEAD".to_string())
    } else {
        branch
    };

    println!("    {:<14}{}", "Branch:".dimmed(), branch);

    // Clean/dirty status
    let is_clean = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output()
        .ok()
        .map(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim().is_empty()
        })
        .unwrap_or(false);

    let status_label = if is_clean {
        "clean".green().to_string()
    } else {
        "dirty".yellow().to_string()
    };
    println!("    {:<14}{}", "Status:".dimmed(), status_label);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_count_files_in_dir_empty() {
        let dir = TempDir::new().unwrap();
        assert_eq!(count_files_in_dir(dir.path()), 0);
    }

    #[test]
    fn test_count_files_in_dir_only_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.md"), "a").unwrap();
        fs::write(dir.path().join("b.md"), "b").unwrap();
        fs::write(dir.path().join("c.txt"), "c").unwrap();
        assert_eq!(count_files_in_dir(dir.path()), 3);
    }

    #[test]
    fn test_count_files_in_dir_skips_directories() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.md"), "content").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        assert_eq!(count_files_in_dir(dir.path()), 1);
    }

    #[test]
    fn test_count_files_in_dir_nonexistent() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert_eq!(count_files_in_dir(&missing), 0);
    }

    #[test]
    fn test_status_requires_harness_dir() {
        // Running status in an empty dir should not panic
        let dir = TempDir::new().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = status(StatusOptions {});
        std::env::set_current_dir(original).unwrap();
        assert!(result.is_ok());
    }
}
