//! Update command -- update Harness CLI configuration to latest version.
//!
//! Ported from `packages/cli/src/commands/update.ts`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{Confirm, Select};

use crate::constants::paths::{constructed, dir_names};
use crate::constants::version::{PACKAGE_NAME, VERSION};
use crate::migrations::{
    get_all_migrations, get_migration_metadata, get_migrations_for_version, MigrationMetadata,
};
use crate::types::migration::{
    ClassifiedMigrations, MigrationAction, MigrationItem, MigrationResult, MigrationType,
    TemplateHashes,
};
use crate::utils::compare_versions::compare_versions;
use crate::utils::proxy::setup_proxy;
use crate::utils::template_hash::{
    compute_hash, is_template_modified, load_hashes, remove_hash, rename_hash, save_hashes,
    update_hashes,
};

// =============================================================================
// UpdateOptions
// =============================================================================

/// Options for the `update` command.
pub struct UpdateOptions {
    pub dry_run: bool,
    pub force: bool,
    pub skip_all: bool,
    pub create_new: bool,
    pub allow_downgrade: bool,
    pub migrate: bool,
}

// =============================================================================
// Internal types
// =============================================================================

#[derive(Debug, Clone)]
struct FileChange {
    path: PathBuf,
    relative_path: String,
    new_content: String,
    #[allow(dead_code)]
    status: FileStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum FileStatus {
    New,
    Unchanged,
    Changed,
}

struct ChangeAnalysis {
    new_files: Vec<FileChange>,
    unchanged_files: Vec<FileChange>,
    auto_update_files: Vec<FileChange>,
    changed_files: Vec<FileChange>,
    user_deleted_files: Vec<FileChange>,
    protected_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConflictAction {
    Overwrite,
    Skip,
    CreateNew,
}

/// Classified safe-file-delete item with reason.
struct SafeFileDeleteClassified {
    item: MigrationItem,
    action: SafeDeleteAction,
}

#[derive(PartialEq)]
enum SafeDeleteAction {
    Delete,
    SkipMissing,
    SkipModified,
    SkipProtected,
    SkipUpdateSkip,
}

// =============================================================================
// Protected paths
// =============================================================================

fn get_protected_paths() -> Vec<String> {
    vec![
        format!(
            "{}/{}",
            dir_names::WORKFLOW,
            dir_names::WORKSPACE
        ),
        format!("{}/{}", dir_names::WORKFLOW, dir_names::TASKS),
        format!("{}/{}", dir_names::WORKFLOW, dir_names::SPEC),
        format!("{}/.developer", dir_names::WORKFLOW),
        format!("{}/.current-task", dir_names::WORKFLOW),
    ]
}

fn is_protected_path(file_path: &str) -> bool {
    let protected = get_protected_paths();
    protected.iter().any(|pp| {
        file_path == pp
            || file_path.starts_with(&format!("{}/", pp))
    })
}

// =============================================================================
// Safe-file-delete
// =============================================================================

fn collect_safe_file_deletes(
    migrations: &[MigrationItem],
    cwd: &Path,
    skip_paths: &[String],
) -> Vec<SafeFileDeleteClassified> {
    let safe_deletes: Vec<&MigrationItem> = migrations
        .iter()
        .filter(|m| matches!(m.type_, MigrationType::SafeFileDelete))
        .collect();

    let mut results = Vec::new();

    for item in safe_deletes {
        let full_path = cwd.join(&item.from);

        if !full_path.exists() {
            results.push(SafeFileDeleteClassified {
                item: item.clone(),
                action: SafeDeleteAction::SkipMissing,
            });
            continue;
        }

        if is_protected_path(&item.from) {
            results.push(SafeFileDeleteClassified {
                item: item.clone(),
                action: SafeDeleteAction::SkipProtected,
            });
            continue;
        }

        if skip_paths.iter().any(|skip| {
            item.from == *skip
                || item
                    .from
                    .starts_with(&format!("{}/", skip.trim_end_matches('/')))
        }) {
            results.push(SafeFileDeleteClassified {
                item: item.clone(),
                action: SafeDeleteAction::SkipUpdateSkip,
            });
            continue;
        }

        if item.allowed_hashes.is_none()
            || item.allowed_hashes.as_ref().map(|h| h.is_empty()).unwrap_or(true)
        {
            results.push(SafeFileDeleteClassified {
                item: item.clone(),
                action: SafeDeleteAction::SkipModified,
            });
            continue;
        }

        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                let file_hash = compute_hash(&content);
                if item
                    .allowed_hashes
                    .as_ref()
                    .unwrap()
                    .contains(&file_hash)
                {
                    results.push(SafeFileDeleteClassified {
                        item: item.clone(),
                        action: SafeDeleteAction::Delete,
                    });
                } else {
                    results.push(SafeFileDeleteClassified {
                        item: item.clone(),
                        action: SafeDeleteAction::SkipModified,
                    });
                }
            }
            Err(_) => {
                results.push(SafeFileDeleteClassified {
                    item: item.clone(),
                    action: SafeDeleteAction::SkipMissing,
                });
            }
        }
    }

    results
}

fn print_safe_file_delete_summary(classified: &[SafeFileDeleteClassified]) {
    let to_delete: Vec<&SafeFileDeleteClassified> = classified
        .iter()
        .filter(|c| c.action == SafeDeleteAction::Delete)
        .collect();
    let modified: Vec<&SafeFileDeleteClassified> = classified
        .iter()
        .filter(|c| c.action == SafeDeleteAction::SkipModified)
        .collect();
    let update_skip: Vec<&SafeFileDeleteClassified> = classified
        .iter()
        .filter(|c| c.action == SafeDeleteAction::SkipUpdateSkip)
        .collect();

    if to_delete.is_empty() && modified.is_empty() && update_skip.is_empty() {
        return;
    }

    println!("{}", "  Deprecated commands cleanup:".cyan());

    for c in &to_delete {
        let desc = c
            .item
            .description
            .as_ref()
            .map(|d| format!(" ({})", d))
            .unwrap_or_default();
        println!(
            "{}",
            format!("    x {}{}", c.item.from, desc).green()
        );
    }

    for c in &modified {
        println!(
            "{}",
            format!("    ? {} (modified, skipped)", c.item.from).yellow()
        );
    }

    for c in &update_skip {
        println!(
            "{}",
            format!("    o {} (skipped, update.skip)", c.item.from).dimmed()
        );
    }

    println!();
}

fn execute_safe_file_deletes(classified: &[SafeFileDeleteClassified], cwd: &Path) -> u32 {
    let to_delete: Vec<&SafeFileDeleteClassified> = classified
        .iter()
        .filter(|c| c.action == SafeDeleteAction::Delete)
        .collect();

    let mut deleted = 0u32;

    for c in to_delete {
        let full_path = cwd.join(&c.item.from);
        if std::fs::remove_file(&full_path).is_ok() {
            remove_hash(cwd, &c.item.from);
            cleanup_empty_dirs(cwd, Path::new(&c.item.from).parent().unwrap_or(Path::new("")));
            deleted += 1;
        }
    }

    deleted
}

// =============================================================================
// Update skip paths (from config.yaml)
// =============================================================================

fn load_update_skip_paths(cwd: &Path) -> Vec<String> {
    let config_path = cwd.join(dir_names::WORKFLOW).join("config.yaml");
    if !config_path.exists() {
        return Vec::new();
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut paths = Vec::new();
    let mut in_update = false;
    let mut in_skip = false;

    let re_update = regex::Regex::new(r"^update:\s*$").unwrap();
    let re_skip = regex::Regex::new(r"^\s+skip:\s*$").unwrap();
    let re_list_item = regex::Regex::new(r"^\s+-\s+(.+)$").unwrap();

    for line in content.lines() {
        let trimmed = line.trim_end();

        if re_update.is_match(trimmed) {
            in_update = true;
            in_skip = false;
            continue;
        }

        if in_update && re_skip.is_match(trimmed) {
            in_skip = true;
            continue;
        }

        if in_skip {
            if let Some(caps) = re_list_item.captures(trimmed) {
                let val = caps[1]
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"')
                    .to_string();
                paths.push(val);
                continue;
            }
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                in_skip = false;
                in_update = false;
            }
        }

        if in_update
            && !trimmed.is_empty()
            && !trimmed.starts_with(' ')
            && !trimmed.starts_with('#')
        {
            in_update = false;
            in_skip = false;
        }
    }

    paths
}

// =============================================================================
// Template file collection (stub)
// =============================================================================

/// Collect all template files that should be managed by update.
///
/// Collects scripts, configuration files, and platform-specific templates
/// for all configured platforms.
fn collect_template_files(cwd: &Path) -> HashMap<String, String> {
    let mut files = HashMap::new();

    // Collect harness-cli scripts from embedded templates
    use crate::templates::extract::{get_embedded_file, list_files, HarnessCliTemplates};

    for file_path in list_files::<HarnessCliTemplates>() {
        if let Some(content) = get_embedded_file::<HarnessCliTemplates>(&file_path) {
            // Scripts go under .harness-cli/scripts/
            if file_path.ends_with(".py") || file_path.ends_with(".sh") {
                files.insert(
                    format!("{}/{}", constructed::SCRIPTS, file_path),
                    content,
                );
            } else {
                files.insert(
                    format!("{}/{}", dir_names::WORKFLOW, file_path),
                    content,
                );
            }
        }
    }

    // Platform-specific templates (only for configured platforms)
    let platforms = crate::configurators::get_configured_platforms(cwd);
    for platform_id in &platforms {
        if let Some(platform_files) = crate::configurators::collect_platform_templates(*platform_id)
        {
            for (file_path, content) in platform_files {
                files.insert(file_path, content);
            }
        }
    }

    // Apply update.skip from config.yaml
    let skip_paths = load_update_skip_paths(cwd);
    if !skip_paths.is_empty() {
        let to_remove: Vec<String> = files
            .keys()
            .filter(|file_path| {
                skip_paths.iter().any(|skip| {
                    *file_path == skip
                        || file_path.starts_with(&format!(
                            "{}/",
                            skip.trim_end_matches('/')
                        ))
                })
            })
            .cloned()
            .collect();
        for key in to_remove {
            files.remove(&key);
        }
    }

    files
}

// =============================================================================
// Change analysis
// =============================================================================

fn analyze_changes(
    cwd: &Path,
    hashes: &TemplateHashes,
    templates: &HashMap<String, String>,
) -> ChangeAnalysis {
    let protected = get_protected_paths();
    let mut result = ChangeAnalysis {
        new_files: Vec::new(),
        unchanged_files: Vec::new(),
        auto_update_files: Vec::new(),
        changed_files: Vec::new(),
        user_deleted_files: Vec::new(),
        protected_paths: protected,
    };

    for (relative_path, new_content) in templates {
        let full_path = cwd.join(relative_path);
        let exists = full_path.exists();

        let change = FileChange {
            path: full_path.clone(),
            relative_path: relative_path.clone(),
            new_content: new_content.clone(),
            status: FileStatus::New,
        };

        if !exists {
            if hashes.contains_key(relative_path) {
                // Previously installed but user deleted -- respect deletion
                result.user_deleted_files.push(change);
            } else {
                result.new_files.push(change);
            }
        } else {
            let existing_content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            if existing_content == *new_content {
                let mut c = change;
                c.status = FileStatus::Unchanged;
                result.unchanged_files.push(c);
            } else {
                let stored_hash = hashes.get(relative_path);
                let current_hash = compute_hash(&existing_content);

                if stored_hash.map(|h| *h == current_hash).unwrap_or(false) {
                    // User didn't modify, template was updated -- safe to auto-update
                    let mut c = change;
                    c.status = FileStatus::Changed;
                    result.auto_update_files.push(c);
                } else {
                    // User modified the file -- needs confirmation
                    let mut c = change;
                    c.status = FileStatus::Changed;
                    result.changed_files.push(c);
                }
            }
        }
    }

    result
}

fn print_change_summary(changes: &ChangeAnalysis) {
    println!("\nScanning for changes...\n");

    if !changes.new_files.is_empty() {
        println!("{}", "  New files (will add):".green());
        for file in &changes.new_files {
            println!("{}", format!("    + {}", file.relative_path).green());
        }
        println!();
    }

    if !changes.auto_update_files.is_empty() {
        println!("{}", "  Template updated (will auto-update):".cyan());
        for file in &changes.auto_update_files {
            println!(
                "{}",
                format!("    {} {}", '\u{2191}', file.relative_path).cyan()
            );
        }
        println!();
    }

    if !changes.unchanged_files.is_empty() {
        println!("{}", "  Unchanged files (will skip):".dimmed());
        for file in changes.unchanged_files.iter().take(5) {
            println!(
                "{}",
                format!("    o {}", file.relative_path).dimmed()
            );
        }
        if changes.unchanged_files.len() > 5 {
            println!(
                "{}",
                format!(
                    "    ... and {} more",
                    changes.unchanged_files.len() - 5
                )
                .dimmed()
            );
        }
        println!();
    }

    if !changes.changed_files.is_empty() {
        println!(
            "{}",
            "  Modified by you (need your decision):".yellow()
        );
        for file in &changes.changed_files {
            println!(
                "{}",
                format!("    ? {}", file.relative_path).yellow()
            );
        }
        println!();
    }

    if !changes.user_deleted_files.is_empty() {
        println!("{}", "  Deleted by you (preserved):".dimmed());
        for file in &changes.user_deleted_files {
            println!(
                "{}",
                format!("    x {}", file.relative_path).dimmed()
            );
        }
        println!();
    }

    // Only show protected paths that actually exist
    let cwd = std::env::current_dir().unwrap_or_default();
    let existing_protected: Vec<&String> = changes
        .protected_paths
        .iter()
        .filter(|p| cwd.join(p).exists())
        .collect();

    if !existing_protected.is_empty() {
        println!("{}", "  User data (preserved):".dimmed());
        for p in &existing_protected {
            println!("{}", format!("    o {}/", p).dimmed());
        }
        println!();
    }
}

// =============================================================================
// Conflict resolution
// =============================================================================

fn prompt_conflict_resolution(
    file: &FileChange,
    options: &UpdateOptions,
    apply_to_all: &mut Option<ConflictAction>,
) -> ConflictAction {
    if let Some(action) = apply_to_all {
        return *action;
    }

    if options.force {
        return ConflictAction::Overwrite;
    }
    if options.skip_all {
        return ConflictAction::Skip;
    }
    if options.create_new {
        return ConflictAction::CreateNew;
    }

    let choices = &[
        "[1] Overwrite - Replace with new version",
        "[2] Skip - Keep your current version",
        "[3] Create copy - Save new version as .new",
        "[a] Apply Overwrite to all",
        "[s] Apply Skip to all",
        "[n] Apply Create copy to all",
    ];

    let selection = Select::new()
        .with_prompt(format!("{} has changes.", file.relative_path))
        .items(choices)
        .default(1)
        .interact()
        .unwrap_or(1);

    match selection {
        0 => ConflictAction::Overwrite,
        1 => ConflictAction::Skip,
        2 => ConflictAction::CreateNew,
        3 => {
            *apply_to_all = Some(ConflictAction::Overwrite);
            ConflictAction::Overwrite
        }
        4 => {
            *apply_to_all = Some(ConflictAction::Skip);
            ConflictAction::Skip
        }
        5 => {
            *apply_to_all = Some(ConflictAction::CreateNew);
            ConflictAction::CreateNew
        }
        _ => ConflictAction::Skip,
    }
}

// =============================================================================
// Backup
// =============================================================================

fn create_backup_dir_path(cwd: &Path) -> PathBuf {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO-like timestamp
    let timestamp = format!("{}", secs);
    cwd.join(dir_names::WORKFLOW)
        .join(format!(".backup-{}", timestamp))
}

fn backup_file(cwd: &Path, backup_dir: &Path, relative_path: &str) {
    let src_path = cwd.join(relative_path);
    if !src_path.exists() {
        return;
    }

    let backup_path = backup_dir.join(relative_path);
    if let Some(parent) = backup_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::copy(&src_path, &backup_path);
}

/// Patterns to exclude from backup.
const BACKUP_EXCLUDE_PATTERNS: &[&str] = &[
    ".backup-",
    "/workspace/",
    "/tasks/",
    "/spec/",
    "/backlog/",
    "/agent-traces/",
];

fn should_exclude_from_backup(relative_path: &str) -> bool {
    BACKUP_EXCLUDE_PATTERNS
        .iter()
        .any(|pattern| relative_path.contains(pattern))
}

fn collect_all_files(dir_path: &Path) -> Vec<PathBuf> {
    if !dir_path.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    let entries = match std::fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_all_files(&path));
        } else if path.is_file() {
            files.push(path);
        }
    }

    files
}

fn create_full_backup(cwd: &Path) -> Option<PathBuf> {
    let backup_dir = create_backup_dir_path(cwd);

    // Backup all managed directories
    let all_managed = crate::configurators::all_managed_dirs();
    let mut has_files = false;

    for dir in &all_managed {
        let dir_path = cwd.join(dir);
        if !dir_path.exists() {
            continue;
        }

        let files = collect_all_files(&dir_path);
        for full_path in files {
            let relative_path = match full_path.strip_prefix(cwd) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            if should_exclude_from_backup(&relative_path) {
                continue;
            }

            if !has_files {
                let _ = std::fs::create_dir_all(&backup_dir);
                has_files = true;
            }
            backup_file(cwd, &backup_dir, &relative_path);
        }
    }

    if has_files {
        Some(backup_dir)
    } else {
        None
    }
}

// =============================================================================
// Version helpers
// =============================================================================

fn update_version_file(cwd: &Path) {
    let version_path = cwd.join(dir_names::WORKFLOW).join(".version");
    let _ = std::fs::write(version_path, VERSION);
}

fn get_installed_version(cwd: &Path) -> String {
    let version_path = cwd.join(dir_names::WORKFLOW).join(".version");
    if version_path.exists() {
        std::fs::read_to_string(version_path)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string()
    } else {
        "unknown".to_string()
    }
}

fn get_latest_npm_version() -> Option<String> {
    let url = format!("https://registry.npmjs.org/{}/latest", PACKAGE_NAME);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: serde_json::Value = resp.json().ok()?;
    data.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// =============================================================================
// Migration classification
// =============================================================================

fn classify_migrations(
    migrations: &[MigrationItem],
    cwd: &Path,
    hashes: &TemplateHashes,
    templates: &HashMap<String, String>,
) -> ClassifiedMigrations {
    let mut result = ClassifiedMigrations::default();

    for item in migrations {
        if matches!(item.type_, MigrationType::SafeFileDelete) {
            continue;
        }

        if is_protected_path(&item.from) {
            result.skip.push(item.clone());
            continue;
        }

        if let Some(ref to) = item.to {
            if is_protected_path(to)
                && !matches!(item.type_, MigrationType::Rename | MigrationType::RenameDir)
            {
                result.skip.push(item.clone());
                continue;
            }
        }

        let old_path = cwd.join(&item.from);
        if !old_path.exists() {
            result.skip.push(item.clone());
            continue;
        }

        match item.type_ {
            MigrationType::Rename => {
                if let Some(ref to) = item.to {
                    let new_path = cwd.join(to);
                    if new_path.exists() {
                        // Check if new file is just template content
                        if is_file_safe_to_replace(cwd, to, templates) {
                            result.auto.push(item.clone());
                        } else {
                            result.conflict.push(item.clone());
                        }
                    } else if is_template_modified(cwd, &item.from, hashes) {
                        result.confirm.push(item.clone());
                    } else {
                        result.auto.push(item.clone());
                    }
                }
            }
            MigrationType::RenameDir => {
                if let Some(ref to) = item.to {
                    let new_path = cwd.join(to);
                    if new_path.exists() {
                        if is_directory_safe_to_replace(cwd, to, hashes, templates) {
                            result.auto.push(item.clone());
                        } else {
                            result.conflict.push(item.clone());
                        }
                    } else {
                        result.auto.push(item.clone());
                    }
                }
            }
            MigrationType::Delete => {
                if is_template_modified(cwd, &item.from, hashes) {
                    result.confirm.push(item.clone());
                } else {
                    result.auto.push(item.clone());
                }
            }
            MigrationType::SafeFileDelete => {
                // Already handled separately
            }
        }
    }

    result
}

fn is_file_safe_to_replace(
    cwd: &Path,
    relative_path: &str,
    templates: &HashMap<String, String>,
) -> bool {
    let full_path = cwd.join(relative_path);
    if !full_path.exists() {
        return true;
    }
    if let Some(template_content) = templates.get(relative_path) {
        if let Ok(current_content) = std::fs::read_to_string(&full_path) {
            return current_content == *template_content;
        }
    }
    false
}

fn is_directory_safe_to_replace(
    cwd: &Path,
    dir_relative_path: &str,
    hashes: &TemplateHashes,
    templates: &HashMap<String, String>,
) -> bool {
    let dir_full_path = cwd.join(dir_relative_path);
    if !dir_full_path.exists() {
        return true;
    }

    let files = collect_all_files(&dir_full_path);
    if files.is_empty() {
        return true;
    }

    for full_path in &files {
        let relative_path = match full_path.strip_prefix(cwd) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        if let Some(template_content) = templates.get(&relative_path) {
            if let Ok(current_content) = std::fs::read_to_string(full_path) {
                if current_content == *template_content {
                    continue;
                }
            }
        }

        if hashes.contains_key(&relative_path)
            && !is_template_modified(cwd, &relative_path, hashes)
        {
            continue;
        }

        return false;
    }

    true
}

// =============================================================================
// Migration execution
// =============================================================================

fn cleanup_empty_dirs(cwd: &Path, dir_path: &Path) {
    let dir_str = dir_path.to_string_lossy().to_string();
    if dir_str.is_empty() || dir_str == "." {
        return;
    }

    let full_path = cwd.join(&dir_str);
    if !full_path.exists() || !full_path.is_dir() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(&full_path) {
        let count = entries.count();
        if count == 0 {
            let _ = std::fs::remove_dir(&full_path);
            if let Some(parent) = Path::new(&dir_str).parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if parent_str != "." && !parent_str.is_empty() {
                    cleanup_empty_dirs(cwd, parent);
                }
            }
        }
    }
}

fn sort_migrations_for_execution(migrations: &[MigrationItem]) -> Vec<MigrationItem> {
    let mut sorted = migrations.to_vec();
    sorted.sort_by(|a, b| {
        if matches!(a.type_, MigrationType::RenameDir)
            && matches!(b.type_, MigrationType::RenameDir)
        {
            let a_depth = a.from.split('/').count();
            let b_depth = b.from.split('/').count();
            b_depth.cmp(&a_depth)
        } else if matches!(a.type_, MigrationType::RenameDir) {
            std::cmp::Ordering::Less
        } else if matches!(b.type_, MigrationType::RenameDir) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    sorted
}

fn print_migration_summary(classified: &ClassifiedMigrations) {
    let total = classified.auto.len()
        + classified.confirm.len()
        + classified.conflict.len()
        + classified.skip.len();

    if total == 0 {
        println!("{}", "  No migrations to apply.\n".dimmed());
        return;
    }

    if !classified.auto.is_empty() {
        println!("{}", "  v Auto-migrate (unmodified):".green());
        for item in &classified.auto {
            match item.type_ {
                MigrationType::Rename => {
                    println!(
                        "{}",
                        format!(
                            "    {} -> {}",
                            item.from,
                            item.to.as_deref().unwrap_or("?")
                        )
                        .green()
                    );
                }
                MigrationType::RenameDir => {
                    println!(
                        "{}",
                        format!(
                            "    [dir] {}/ -> {}/",
                            item.from,
                            item.to.as_deref().unwrap_or("?")
                        )
                        .green()
                    );
                }
                _ => {
                    println!(
                        "{}",
                        format!("    x {}", item.from).green()
                    );
                }
            }
        }
        println!();
    }

    if !classified.confirm.is_empty() {
        println!(
            "{}",
            "  ! Requires confirmation (modified by user):".yellow()
        );
        for item in &classified.confirm {
            match item.type_ {
                MigrationType::Rename => {
                    println!(
                        "{}",
                        format!(
                            "    {} -> {}",
                            item.from,
                            item.to.as_deref().unwrap_or("?")
                        )
                        .yellow()
                    );
                }
                _ => {
                    println!(
                        "{}",
                        format!("    x {}", item.from).yellow()
                    );
                }
            }
        }
        println!();
    }

    if !classified.conflict.is_empty() {
        println!(
            "{}",
            "  Conflict (both old and new exist):".red()
        );
        for item in &classified.conflict {
            match item.type_ {
                MigrationType::RenameDir => {
                    println!(
                        "{}",
                        format!(
                            "    [dir] {}/ <-> {}/",
                            item.from,
                            item.to.as_deref().unwrap_or("?")
                        )
                        .red()
                    );
                }
                _ => {
                    println!(
                        "{}",
                        format!(
                            "    {} <-> {}",
                            item.from,
                            item.to.as_deref().unwrap_or("?")
                        )
                        .red()
                    );
                }
            }
        }
        println!(
            "{}",
            "    -> Resolve manually: merge or delete one, then re-run update".dimmed()
        );
        println!();
    }

    if !classified.skip.is_empty() {
        println!("{}", "  o Skipping (old file not found):".dimmed());
        for item in classified.skip.iter().take(3) {
            println!("{}", format!("    {}", item.from).dimmed());
        }
        if classified.skip.len() > 3 {
            println!(
                "{}",
                format!("    ... and {} more", classified.skip.len() - 3).dimmed()
            );
        }
        println!();
    }
}

fn prompt_migration_action(item: &MigrationItem) -> MigrationAction {
    let action_desc = if matches!(item.type_, MigrationType::Rename) {
        format!("{} -> {}", item.from, item.to.as_deref().unwrap_or("?"))
    } else {
        format!("Delete {}", item.from)
    };

    let rename_label = if matches!(item.type_, MigrationType::Rename) {
        "[r] Rename anyway"
    } else {
        "[d] Delete anyway"
    };

    let choices = &[
        rename_label,
        "[b] Backup original, then proceed",
        "[s] Skip this migration",
    ];

    let selection = Select::new()
        .with_prompt(format!(
            "{}\nThis file has been modified. What would you like to do?",
            action_desc
        ))
        .items(choices)
        .default(2)
        .interact()
        .unwrap_or(2);

    match selection {
        0 => MigrationAction::Rename,
        1 => MigrationAction::BackupRename,
        _ => MigrationAction::Skip,
    }
}

fn execute_migrations(
    classified: &ClassifiedMigrations,
    cwd: &Path,
    force: bool,
    skip_all: bool,
) -> MigrationResult {
    let mut result = MigrationResult {
        renamed: 0,
        deleted: 0,
        skipped: 0,
        conflicts: classified.conflict.len() as u32,
    };

    let sorted_auto = sort_migrations_for_execution(&classified.auto);

    // 1. Execute auto migrations
    for item in &sorted_auto {
        match item.type_ {
            MigrationType::Rename => {
                if let Some(ref to) = item.to {
                    let old_path = cwd.join(&item.from);
                    let new_path = cwd.join(to);

                    if let Some(parent) = new_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::rename(&old_path, &new_path).is_ok() {
                        rename_hash(cwd, &item.from, to);

                        #[cfg(unix)]
                        if to.ends_with(".sh") || to.ends_with(".py") {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(metadata) = std::fs::metadata(&new_path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(0o755);
                                let _ = std::fs::set_permissions(&new_path, perms);
                            }
                        }

                        if let Some(parent) = Path::new(&item.from).parent() {
                            cleanup_empty_dirs(cwd, parent);
                        }
                        result.renamed += 1;
                    }
                }
            }
            MigrationType::RenameDir => {
                if let Some(ref to) = item.to {
                    let old_path = cwd.join(&item.from);
                    let new_path = cwd.join(to);

                    if new_path.exists() {
                        let _ = std::fs::remove_dir_all(&new_path);
                    }

                    if let Some(parent) = new_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }

                    if std::fs::rename(&old_path, &new_path).is_ok() {
                        // Batch update hash tracking
                        let hashes = load_hashes(cwd);
                        let old_prefix = if item.from.ends_with('/') {
                            item.from.clone()
                        } else {
                            format!("{}/", item.from)
                        };
                        let new_prefix = if to.ends_with('/') {
                            to.clone()
                        } else {
                            format!("{}/", to)
                        };

                        let mut updated_hashes: TemplateHashes = HashMap::new();
                        for (hash_path, hash_value) in &hashes {
                            if hash_path.starts_with(&old_prefix) {
                                let new_hash_path = format!(
                                    "{}{}",
                                    new_prefix,
                                    &hash_path[old_prefix.len()..]
                                );
                                updated_hashes.insert(new_hash_path, hash_value.clone());
                            } else if hash_path.starts_with(&new_prefix) {
                                continue; // Skip old hashes from deleted target
                            } else {
                                updated_hashes.insert(hash_path.clone(), hash_value.clone());
                            }
                        }
                        save_hashes(cwd, &updated_hashes);

                        result.renamed += 1;
                    }
                }
            }
            MigrationType::Delete => {
                let file_path = cwd.join(&item.from);
                if std::fs::remove_file(&file_path).is_ok() {
                    remove_hash(cwd, &item.from);
                    if let Some(parent) = Path::new(&item.from).parent() {
                        cleanup_empty_dirs(cwd, parent);
                    }
                    result.deleted += 1;
                }
            }
            MigrationType::SafeFileDelete => {
                // Handled separately
            }
        }
    }

    // 2. Handle confirm items (modified files)
    for item in &classified.confirm {
        let action = if force {
            MigrationAction::Rename
        } else if skip_all {
            MigrationAction::Skip
        } else {
            prompt_migration_action(item)
        };

        if action == MigrationAction::Skip {
            result.skipped += 1;
            continue;
        }

        match item.type_ {
            MigrationType::Rename => {
                if let Some(ref to) = item.to {
                    let old_path = cwd.join(&item.from);
                    let new_path = cwd.join(to);

                    if let Some(parent) = new_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::rename(&old_path, &new_path).is_ok() {
                        rename_hash(cwd, &item.from, to);

                        #[cfg(unix)]
                        if to.ends_with(".sh") || to.ends_with(".py") {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(metadata) = std::fs::metadata(&new_path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(0o755);
                                let _ = std::fs::set_permissions(&new_path, perms);
                            }
                        }

                        if let Some(parent) = Path::new(&item.from).parent() {
                            cleanup_empty_dirs(cwd, parent);
                        }
                        result.renamed += 1;
                    }
                }
            }
            MigrationType::Delete => {
                let file_path = cwd.join(&item.from);
                if std::fs::remove_file(&file_path).is_ok() {
                    remove_hash(cwd, &item.from);
                    if let Some(parent) = Path::new(&item.from).parent() {
                        cleanup_empty_dirs(cwd, parent);
                    }
                    result.deleted += 1;
                }
            }
            _ => {}
        }
    }

    // 3. Skip count
    result.skipped += classified.skip.len() as u32;

    result
}

fn print_migration_result(result: &MigrationResult) {
    let mut parts = Vec::new();
    if result.renamed > 0 {
        parts.push(format!("{} renamed", result.renamed));
    }
    if result.deleted > 0 {
        parts.push(format!("{} deleted", result.deleted));
    }
    if result.skipped > 0 {
        parts.push(format!("{} skipped", result.skipped));
    }
    if result.conflicts > 0 {
        parts.push(format!(
            "{} conflict{}",
            result.conflicts,
            if result.conflicts > 1 { "s" } else { "" }
        ));
    }
    if !parts.is_empty() {
        println!(
            "{}",
            format!("Migration complete: {}", parts.join(", ")).cyan()
        );
    }
}

// =============================================================================
// Needs Codex upgrade detection
// =============================================================================

fn needs_codex_upgrade(cwd: &Path) -> bool {
    if cwd.join(".codex").exists() {
        return false;
    }
    let hashes = load_hashes(cwd);
    hashes.keys().any(|key| key.starts_with(".agents/skills/"))
}

// =============================================================================
// Main update function
// =============================================================================

/// Run the `update` command.
pub fn update(options: UpdateOptions) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Check if Harness CLI is initialized
    if !cwd.join(dir_names::WORKFLOW).exists() {
        println!(
            "{}",
            "Error: Harness CLI not initialized in this directory.".red()
        );
        println!("{}", "Run 'harness-cli init' first.".dimmed());
        return Ok(());
    }

    println!("{}", "\nHarness CLI Update".cyan());
    println!("{}", "==============\n".cyan());

    // Set up proxy
    setup_proxy();

    // Get versions
    let project_version = get_installed_version(&cwd);
    let cli_version = VERSION;
    let latest_npm_version = get_latest_npm_version();

    // Version comparison
    let cli_vs_project = compare_versions(cli_version, &project_version);
    let cli_vs_npm = latest_npm_version
        .as_ref()
        .map(|npm| compare_versions(cli_version, npm))
        .unwrap_or(std::cmp::Ordering::Equal);

    // Display versions
    println!("Project version: {}", project_version.white());
    println!("CLI version:     {}", cli_version.white());
    if let Some(ref npm_ver) = latest_npm_version {
        println!("Latest on npm:   {}", npm_ver.white());
    } else {
        println!("{}", "Latest on npm:   (unable to fetch)".dimmed());
    }
    println!();

    // Check if CLI is outdated
    if cli_vs_npm == std::cmp::Ordering::Less {
        if let Some(ref npm_ver) = latest_npm_version {
            println!(
                "{}",
                format!(
                    "  Your CLI ({}) is behind npm ({}).",
                    cli_version, npm_ver
                )
                .yellow()
            );
            println!(
                "{}",
                format!("   Run: npm install -g {}\n", PACKAGE_NAME).yellow()
            );
        }
    }

    // Check for downgrade
    if cli_vs_project == std::cmp::Ordering::Less {
        println!(
            "{}",
            format!(
                "  Cannot update: CLI version ({}) < project version ({})",
                cli_version, project_version
            )
            .red()
        );
        println!(
            "{}",
            "   This would DOWNGRADE your project!\n".red()
        );

        if !options.allow_downgrade {
            println!("{}", "Solutions:".dimmed());
            println!(
                "{}",
                format!("  1. Update your CLI: npm install -g {}", PACKAGE_NAME).dimmed()
            );
            println!(
                "{}",
                "  2. Force downgrade: harness-cli update --allow-downgrade\n".dimmed()
            );
            return Ok(());
        }

        println!(
            "{}",
            "  --allow-downgrade flag set. Proceeding with downgrade...\n".yellow()
        );
    }

    // Load template hashes
    let hashes = load_hashes(&cwd);
    let is_first_hash_tracking = hashes.is_empty();

    // Handle unknown version
    let is_unknown_version = project_version == "unknown";
    if is_unknown_version {
        println!(
            "{}",
            "  No version file found. Skipping migrations -- run harness-cli init to fix."
                .yellow()
        );
        println!(
            "{}",
            "   Template updates will still be applied.".dimmed()
        );
        println!(
            "{}",
            "   Safe file cleanup will still run (hash-verified).\n".dimmed()
        );
    }

    // Detect legacy Codex
    let codex_upgrade_needed = needs_codex_upgrade(&cwd);
    if codex_upgrade_needed {
        println!(
            "{}",
            "  Legacy Codex detected: .agents/skills/ tracked without .codex/ -- will create .codex/ directory"
                .yellow()
        );
    }

    // Collect templates
    let templates = collect_template_files(&cwd);

    // Load update.skip paths
    let skip_paths = load_update_skip_paths(&cwd);

    // Collect safe-file-delete items from ALL manifests
    let all_migrations = get_all_migrations();
    let safe_file_deletes = collect_safe_file_deletes(&all_migrations, &cwd, &skip_paths);
    let has_safe_deletes = safe_file_deletes
        .iter()
        .any(|c| c.action == SafeDeleteAction::Delete);

    // Check for pending regular migrations
    let mut pending_migrations = if is_unknown_version {
        Vec::new()
    } else {
        get_migrations_for_version(&project_version, cli_version)
    };

    // Check for orphaned migrations
    let orphaned_migrations: Vec<MigrationItem> = all_migrations
        .iter()
        .filter(|item| {
            if !matches!(item.type_, MigrationType::Rename | MigrationType::RenameDir) {
                return false;
            }
            if item.to.is_none() {
                return false;
            }

            let old_path = cwd.join(&item.from);
            let new_path = cwd.join(item.to.as_ref().unwrap());

            let source_exists = old_path.exists();
            let target_exists = new_path.exists();
            let already_pending = pending_migrations
                .iter()
                .any(|m| m.from == item.from && m.to == item.to);

            source_exists && !target_exists && !already_pending
        })
        .cloned()
        .collect();

    if !orphaned_migrations.is_empty() {
        println!(
            "{}",
            "  Detected incomplete migrations from previous updates:".yellow()
        );
        for item in &orphaned_migrations {
            println!(
                "{}",
                format!(
                    "    {} -> {}",
                    item.from,
                    item.to.as_deref().unwrap_or("?")
                )
                .yellow()
            );
        }
        println!();
        pending_migrations.extend(orphaned_migrations);
    }

    let has_migrations = !pending_migrations.is_empty();

    // Classify migrations
    let classified_migrations = if has_migrations {
        println!("{}", "Analyzing migrations...\n".cyan());
        let classified = classify_migrations(&pending_migrations, &cwd, &hashes, &templates);
        print_migration_summary(&classified);

        if !options.migrate {
            let auto_count = classified.auto.len();
            let confirm_count = classified.confirm.len();
            if auto_count > 0 || confirm_count > 0 {
                println!(
                    "{}",
                    "Tip: Use --migrate to apply migrations (prompts for modified files).".dimmed()
                );
                if confirm_count > 0 {
                    println!(
                        "{}",
                        "     Use --migrate -f to force all, or --migrate -s to skip modified.\n"
                            .dimmed()
                    );
                } else {
                    println!();
                }
            }
        }

        Some(classified)
    } else {
        None
    };

    // Print safe-file-delete summary
    if !safe_file_deletes.is_empty() {
        print_safe_file_delete_summary(&safe_file_deletes);
    }

    // Analyze changes
    let changes = analyze_changes(&cwd, &hashes, &templates);

    // Print summary
    print_change_summary(&changes);

    // First-time hash tracking hint
    if is_first_hash_tracking && !changes.changed_files.is_empty() {
        println!("{}", "  First update with hash tracking enabled.".cyan());
        println!(
            "{}",
            "   Changed files shown above may not be actual user modifications.".dimmed()
        );
        println!(
            "{}",
            "   After this update, hash tracking will accurately detect changes.\n".dimmed()
        );
    }

    // Check if there's anything to do
    let is_upgrade = cli_vs_project == std::cmp::Ordering::Greater;
    let is_downgrade = cli_vs_project == std::cmp::Ordering::Less;
    let is_same_version = cli_vs_project == std::cmp::Ordering::Equal;

    let has_pending = options.migrate
        && classified_migrations
            .as_ref()
            .is_some_and(|c| !c.auto.is_empty() || !c.confirm.is_empty());

    if changes.new_files.is_empty()
        && changes.auto_update_files.is_empty()
        && changes.changed_files.is_empty()
        && !has_pending
        && !has_safe_deletes
    {
        if is_same_version {
            println!("{}", "v Already up to date!".green());
        } else {
            update_version_file(&cwd);
            if is_upgrade {
                println!(
                    "{}",
                    format!(
                        "v No file changes needed for {} -> {}",
                        project_version, cli_version
                    )
                    .green()
                );
            } else if is_downgrade {
                println!(
                    "{}",
                    format!(
                        "v No file changes needed for {} -> {} (downgrade)",
                        project_version, cli_version
                    )
                    .green()
                );
            }
        }
        return Ok(());
    }

    // Show what this operation will do
    if is_upgrade {
        println!(
            "{}",
            format!("This will UPGRADE: {} -> {}\n", project_version, cli_version).green()
        );
    } else if is_downgrade {
        println!(
            "{}",
            format!(
                "  This will DOWNGRADE: {} -> {}\n",
                project_version, cli_version
            )
            .red()
        );
    }

    // Show breaking change warning before confirm
    if cli_vs_project == std::cmp::Ordering::Greater && project_version != "unknown" {
        let pre_metadata = get_migration_metadata(&project_version, cli_version);
        if pre_metadata.breaking {
            println!("{}", "=".repeat(60).cyan());
            println!(
                "{}{}",
                " !! BREAKING CHANGES ".on_red().white().bold(),
                " Review the changes above carefully!".red().bold()
            );
            if !pre_metadata.changelog.is_empty() {
                println!();
                println!("{}", pre_metadata.changelog[0].white());
            }
            if pre_metadata.recommend_migrate && !options.migrate {
                println!();
                println!(
                    "{}{}",
                    " RECOMMENDED ".on_green().black().bold(),
                    " Run with --migrate to complete the migration".green().bold()
                );
            }
            println!("{}", "=".repeat(60).cyan());
            println!();
        }
    }

    // Dry run mode
    if options.dry_run {
        println!("{}", "[Dry run] No changes made.".dimmed());
        return Ok(());
    }

    // Confirm
    let proceed = Confirm::new()
        .with_prompt("Proceed?")
        .default(true)
        .interact()
        .unwrap_or(false);

    if !proceed {
        println!("{}", "Update cancelled.".yellow());
        return Ok(());
    }

    // Create backup
    let backup_dir = create_full_backup(&cwd);
    if let Some(ref bd) = backup_dir {
        let rel_path = bd
            .strip_prefix(&cwd)
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_else(|_| bd.to_string_lossy().to_string());
        println!("{}", format!("\nBackup created: {}/", rel_path).dimmed());
    }

    // Execute migrations if --migrate flag is set
    if options.migrate {
        if let Some(ref classified) = classified_migrations {
            let migration_result = execute_migrations(
                classified,
                &cwd,
                options.force,
                options.skip_all,
            );
            print_migration_result(&migration_result);

            // Hardcoded: Rename traces-*.md to journal-*.md in workspace directories
            let workspace_dir = cwd.join(constructed::WORKSPACE);
            if workspace_dir.exists() {
                let mut journal_renamed = 0u32;
                if let Ok(dev_dirs) = std::fs::read_dir(&workspace_dir) {
                    for dev_entry in dev_dirs.filter_map(|e| e.ok()) {
                        let dev_path = dev_entry.path();
                        if !dev_path.is_dir() {
                            continue;
                        }
                        if let Ok(files) = std::fs::read_dir(&dev_path) {
                            for file_entry in files.filter_map(|e| e.ok()) {
                                let fname = file_entry.file_name().to_string_lossy().to_string();
                                if fname.starts_with("traces-") && fname.ends_with(".md") {
                                    let old_path = file_entry.path();
                                    let new_file = fname.replace("traces-", "journal-");
                                    let new_path = dev_path.join(new_file);
                                    if std::fs::rename(&old_path, &new_path).is_ok() {
                                        journal_renamed += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                if journal_renamed > 0 {
                    println!(
                        "{}",
                        format!("Renamed {} traces file(s) to journal", journal_renamed).cyan()
                    );
                }
            }
        }
    }

    // Execute safe-file-delete (after backup, before template writes)
    let mut safe_deleted = 0u32;
    if has_safe_deletes {
        safe_deleted = execute_safe_file_deletes(&safe_file_deletes, &cwd);
        if safe_deleted > 0 {
            println!(
                "{}",
                format!("\nCleaned up {} deprecated command file(s)", safe_deleted).cyan()
            );
        }
    }

    // Track results
    let mut added = 0u32;
    let mut auto_updated = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    let mut created_new = 0u32;

    // Add new files
    if !changes.new_files.is_empty() {
        println!("{}", "\nAdding new files...".blue());
        for file in &changes.new_files {
            if let Some(parent) = file.path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&file.path, &file.new_content).is_ok() {
                // Make scripts executable
                #[cfg(unix)]
                if file.relative_path.ends_with(".sh") || file.relative_path.ends_with(".py") {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&file.path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = std::fs::set_permissions(&file.path, perms);
                    }
                }
                println!("{}", format!("  + {}", file.relative_path).green());
                added += 1;
            }
        }
    }

    // Auto-update files
    if !changes.auto_update_files.is_empty() {
        println!("{}", "\nAuto-updating template files...".blue());
        for file in &changes.auto_update_files {
            if std::fs::write(&file.path, &file.new_content).is_ok() {
                #[cfg(unix)]
                if file.relative_path.ends_with(".sh") || file.relative_path.ends_with(".py") {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&file.path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = std::fs::set_permissions(&file.path, perms);
                    }
                }
                println!(
                    "{}",
                    format!("  {} {}", '\u{2191}', file.relative_path).cyan()
                );
                auto_updated += 1;
            }
        }
    }

    // Handle changed files
    if !changes.changed_files.is_empty() {
        println!("{}", "\n--- Resolving conflicts ---\n".blue());

        let mut apply_to_all: Option<ConflictAction> = None;

        for file in &changes.changed_files {
            let action = prompt_conflict_resolution(file, &options, &mut apply_to_all);

            match action {
                ConflictAction::Overwrite => {
                    if std::fs::write(&file.path, &file.new_content).is_ok() {
                        #[cfg(unix)]
                        if file.relative_path.ends_with(".sh")
                            || file.relative_path.ends_with(".py")
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(metadata) = std::fs::metadata(&file.path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(0o755);
                                let _ = std::fs::set_permissions(&file.path, perms);
                            }
                        }
                        println!(
                            "{}",
                            format!("  v Overwritten: {}", file.relative_path).yellow()
                        );
                        updated += 1;
                    }
                }
                ConflictAction::CreateNew => {
                    let new_path = format!("{}.new", file.path.to_string_lossy());
                    if std::fs::write(&new_path, &file.new_content).is_ok() {
                        println!(
                            "{}",
                            format!("  v Created: {}.new", file.relative_path).blue()
                        );
                        created_new += 1;
                    }
                }
                ConflictAction::Skip => {
                    println!(
                        "{}",
                        format!("  o Skipped: {}", file.relative_path).dimmed()
                    );
                    skipped += 1;
                }
            }
        }
    }

    // Update version file
    update_version_file(&cwd);

    // Update template hashes
    let mut files_to_hash: HashMap<String, String> = HashMap::new();
    for file in &changes.new_files {
        files_to_hash.insert(file.relative_path.clone(), file.new_content.clone());
    }
    for file in &changes.auto_update_files {
        files_to_hash.insert(file.relative_path.clone(), file.new_content.clone());
    }
    for file in &changes.changed_files {
        if file.path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file.path) {
                if content == file.new_content {
                    files_to_hash.insert(file.relative_path.clone(), file.new_content.clone());
                }
            }
        }
    }
    if !files_to_hash.is_empty() {
        update_hashes(&cwd, &files_to_hash);
    }

    // Print summary
    println!("{}", "\n--- Summary ---\n".cyan());
    if added > 0 {
        println!("  Added: {} file(s)", added);
    }
    if auto_updated > 0 {
        println!("  Auto-updated: {} file(s)", auto_updated);
    }
    if updated > 0 {
        println!("  Updated: {} file(s)", updated);
    }
    if skipped > 0 {
        println!("  Skipped: {} file(s)", skipped);
    }
    if created_new > 0 {
        println!("  Created .new copies: {} file(s)", created_new);
    }
    if safe_deleted > 0 {
        println!("  Cleaned up: {} deprecated file(s)", safe_deleted);
    }
    if let Some(ref bd) = backup_dir {
        let rel = bd
            .strip_prefix(&cwd)
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_else(|_| bd.to_string_lossy().to_string());
        println!("  Backup: {}/", rel);
    }

    let action_word = if is_downgrade { "Downgrade" } else { "Update" };
    println!(
        "{}",
        format!(
            "\nv {} complete! ({} -> {})",
            action_word, project_version, cli_version
        )
        .green()
    );

    if created_new > 0 {
        println!(
            "{}",
            "\nTip: Review .new files and merge changes manually if needed.".dimmed()
        );
    }

    // Create migration task if there are breaking changes with migration guides
    if cli_vs_project == std::cmp::Ordering::Greater && project_version != "unknown" {
        let metadata = get_migration_metadata(&project_version, cli_version);

        if metadata.breaking && !metadata.migration_guides.is_empty() {
            create_migration_task(&cwd, &project_version, cli_version, &metadata);
        }

        // Display breaking change warnings at the very end
        if metadata.breaking || !metadata.changelog.is_empty() {
            println!();
            println!("{}", "=".repeat(60).cyan());

            if metadata.breaking {
                println!(
                    "{}{}",
                    " !! BREAKING CHANGES ".on_red().white().bold(),
                    " This update contains breaking changes!".red().bold()
                );
                println!();
            }

            if !metadata.changelog.is_empty() {
                println!("{}", "What's Changed:".cyan().bold());
                for entry in &metadata.changelog {
                    println!("   {}", entry.white());
                }
                println!();
            }

            if metadata.recommend_migrate && !options.migrate {
                println!(
                    "{}{}",
                    " RECOMMENDED ".on_green().black().bold(),
                    " Run with --migrate to complete the migration".green().bold()
                );
                println!(
                    "{}",
                    "   This will remove legacy files and apply all changes.".dimmed()
                );
                println!();
            }

            println!("{}", "=".repeat(60).cyan());
        }
    }

    Ok(())
}

// =============================================================================
// Migration task creation
// =============================================================================

fn create_migration_task(
    cwd: &Path,
    project_version: &str,
    cli_version: &str,
    metadata: &MigrationMetadata,
) {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day) = crate::commands::init::chrono_today_parts(secs);

    let month_day = format!("{:02}-{:02}", month, day);
    let task_slug = format!("migrate-to-{}", cli_version);
    let task_dir_name = format!("{}-{}", month_day, task_slug);
    let tasks_dir = cwd.join(dir_names::WORKFLOW).join(dir_names::TASKS);
    let task_dir = tasks_dir.join(&task_dir_name);

    if task_dir.exists() {
        return;
    }

    if std::fs::create_dir_all(&task_dir).is_err() {
        return;
    }

    // Read current developer
    let developer_file = cwd.join(dir_names::WORKFLOW).join(".developer");
    let current_developer = if developer_file.exists() {
        std::fs::read_to_string(&developer_file)
            .ok()
            .and_then(|raw| {
                regex::Regex::new(r"(?m)^\s*name\s*=\s*(.+?)\s*$")
                    .ok()
                    .and_then(|re| re.captures(&raw))
                    .map(|caps| caps[1].to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    let today_str = format!("{:04}-{:02}-{:02}", year, month, day);

    let task_json = serde_json::json!({
        "title": format!("Migrate to v{}", cli_version),
        "description": format!("Breaking change migration from v{} to v{}", project_version, cli_version),
        "status": "planning",
        "dev_type": null,
        "scope": "migration",
        "priority": "P1",
        "creator": "harness-cli-update",
        "assignee": current_developer,
        "createdAt": today_str,
        "completedAt": null,
        "branch": null,
        "base_branch": null,
        "worktree_path": null,
        "current_phase": 0,
        "next_action": [
            {"phase": 1, "action": "review-guide"},
            {"phase": 2, "action": "update-files"},
            {"phase": 3, "action": "run-migrate"},
            {"phase": 4, "action": "test"},
        ],
        "commit": null,
        "pr_url": null,
        "subtasks": [],
        "children": [],
        "parent": null,
        "meta": {},
    });

    let task_json_path = task_dir.join("task.json");
    let _ = std::fs::write(
        &task_json_path,
        serde_json::to_string_pretty(&task_json).unwrap_or_default(),
    );

    // Build PRD content
    let mut prd = format!("# Migration Task: Upgrade to v{}\n\n", cli_version);
    prd += &format!("**Created**: {}\n", today_str);
    prd += &format!("**From Version**: {}\n", project_version);
    prd += &format!("**To Version**: {}\n", cli_version);
    prd += &format!("**Assignee**: {}\n\n", current_developer);
    prd += "## Status\n\n- [ ] Review migration guide\n- [ ] Update custom files\n- [ ] Run `harness-cli update --migrate`\n- [ ] Test workflows\n\n";

    for guide in &metadata.migration_guides {
        prd += &format!("---\n\n## v{} Migration Guide\n\n", guide.version);
        prd += &guide.guide;
        prd += "\n\n";

        if let Some(ref ai_instructions) = guide.ai_instructions {
            prd += "### AI Assistant Instructions\n\n";
            prd += "When helping with this migration:\n\n";
            prd += ai_instructions;
            prd += "\n\n";
        }
    }

    let prd_path = task_dir.join("prd.md");
    let _ = std::fs::write(&prd_path, prd);

    println!();
    println!("{}", " MIGRATION TASK CREATED ".on_cyan().black().bold());
    println!(
        "{}",
        "A task has been created to help you complete the migration:".cyan()
    );
    println!(
        "   {}/{}/{}/",
        dir_names::WORKFLOW,
        dir_names::TASKS,
        task_dir_name
    );
    println!();
    println!(
        "{}",
        "Use AI to help: Ask Claude/Cursor to read the task and fix your custom files.".dimmed()
    );
}
