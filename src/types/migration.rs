//! Migration types for Harness CLI update command
//!
//! These types support intelligent migration during updates,
//! handling file renames, deletions, and user modification detection.

use serde::Deserialize;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// MigrationType
// ---------------------------------------------------------------------------

/// The kind of migration action.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationType {
    Rename,
    RenameDir,
    Delete,
    SafeFileDelete,
}

// ---------------------------------------------------------------------------
// MigrationItem
// ---------------------------------------------------------------------------

/// A single migration action (rename, rename-dir, delete, or safe-file-delete).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MigrationItem {
    /// Type of migration action.
    #[serde(rename = "type")]
    pub type_: MigrationType,

    /// Source path (relative to project root).
    pub from: String,

    /// Target path for renames (relative to project root).
    #[serde(default)]
    pub to: Option<String>,

    /// Human-readable description of the change.
    #[serde(default)]
    pub description: Option<String>,

    /// Known template hashes for safe-file-delete (only delete if content matches).
    #[serde(default)]
    pub allowed_hashes: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// MigrationManifest
// ---------------------------------------------------------------------------

/// Migration manifest for a specific version.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationManifest {
    /// Target version this migration upgrades to.
    pub version: String,

    /// Human-readable description of changes in this version.
    #[serde(default)]
    pub description: Option<String>,

    /// List of migration actions.
    pub migrations: Vec<MigrationItem>,

    /// Detailed changelog for display to users.
    #[serde(default)]
    pub changelog: Option<String>,

    /// Whether this version contains breaking changes.
    #[serde(default)]
    pub breaking: Option<bool>,

    /// Whether users should run `--migrate` (recommended for breaking changes).
    #[serde(default)]
    pub recommend_migrate: Option<bool>,

    /// Detailed migration guide for AI-assisted fixes (markdown format).
    #[serde(default)]
    pub migration_guide: Option<String>,

    /// Instructions for AI assistants on how to help with migration.
    #[serde(default)]
    pub ai_instructions: Option<String>,
}

// ---------------------------------------------------------------------------
// MigrationClassification
// ---------------------------------------------------------------------------

/// Classification of how a migration should be handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationClassification {
    /// Unmodified by user -- can auto-migrate.
    Auto,
    /// Modified by user -- needs confirmation.
    Confirm,
    /// Both old and new files exist.
    Conflict,
    /// Old file doesn't exist -- nothing to do.
    Skip,
}

// ---------------------------------------------------------------------------
// ClassifiedMigrations
// ---------------------------------------------------------------------------

/// Result of classifying all migrations.
#[derive(Clone, Debug, Default)]
pub struct ClassifiedMigrations {
    /// Unmodified files -- safe to auto-migrate.
    pub auto: Vec<MigrationItem>,
    /// User-modified files -- need confirmation.
    pub confirm: Vec<MigrationItem>,
    /// Conflict -- both old and new exist.
    pub conflict: Vec<MigrationItem>,
    /// Skip -- old file doesn't exist.
    pub skip: Vec<MigrationItem>,
}

// ---------------------------------------------------------------------------
// MigrationResult
// ---------------------------------------------------------------------------

/// Result of executing migrations.
#[derive(Clone, Copy, Debug, Default)]
pub struct MigrationResult {
    /// Number of files renamed.
    pub renamed: u32,
    /// Number of files deleted.
    pub deleted: u32,
    /// Number of files skipped (user choice or no action needed).
    pub skipped: u32,
    /// Number of conflicts encountered.
    pub conflicts: u32,
}

// ---------------------------------------------------------------------------
// MigrationAction
// ---------------------------------------------------------------------------

/// User action choice for migration confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationAction {
    /// Proceed with rename anyway.
    Rename,
    /// Backup original, then rename.
    BackupRename,
    /// Skip this migration.
    Skip,
    /// View the diff first.
    ViewDiff,
}

// ---------------------------------------------------------------------------
// TemplateHashes
// ---------------------------------------------------------------------------

/// Maps relative file paths to their SHA256 hashes.
pub type TemplateHashes = HashMap<String, String>;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_migration_item_rename() {
        let json = r#"{
            "type": "rename",
            "from": "old/path.md",
            "to": "new/path.md",
            "description": "Moved file"
        }"#;

        let item: MigrationItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.type_, MigrationType::Rename);
        assert_eq!(item.from, "old/path.md");
        assert_eq!(item.to.as_deref(), Some("new/path.md"));
        assert_eq!(item.description.as_deref(), Some("Moved file"));
        assert!(item.allowed_hashes.is_none());
    }

    #[test]
    fn deserialize_migration_item_safe_file_delete() {
        let json = r#"{
            "type": "safe-file-delete",
            "from": "old/file.md",
            "allowed_hashes": ["abc123", "def456"]
        }"#;

        let item: MigrationItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.type_, MigrationType::SafeFileDelete);
        assert_eq!(
            item.allowed_hashes.as_deref(),
            Some(&["abc123".to_string(), "def456".to_string()][..])
        );
    }

    #[test]
    fn deserialize_migration_manifest() {
        let json = r#"{
            "version": "0.4.0",
            "description": "Major refactor",
            "migrations": [
                {
                    "type": "rename",
                    "from": "a.md",
                    "to": "b.md"
                }
            ],
            "breaking": true,
            "recommendMigrate": true
        }"#;

        let manifest: MigrationManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "0.4.0");
        assert_eq!(manifest.breaking, Some(true));
        assert_eq!(manifest.recommend_migrate, Some(true));
        assert_eq!(manifest.migrations.len(), 1);
    }

    #[test]
    fn migration_result_default() {
        let result = MigrationResult::default();
        assert_eq!(result.renamed, 0);
        assert_eq!(result.deleted, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.conflicts, 0);
    }

    // --- Additional tests ported from TypeScript ---

    #[test]
    fn test_deserialize_rename() {
        let json = r#"{
            "type": "rename",
            "from": "src/old.ts",
            "to": "src/new.ts"
        }"#;
        let item: MigrationItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.type_, MigrationType::Rename);
        assert_eq!(item.from, "src/old.ts");
        assert_eq!(item.to.as_deref(), Some("src/new.ts"));
    }

    #[test]
    fn test_deserialize_safe_file_delete() {
        let json = r#"{
            "type": "safe-file-delete",
            "from": "deprecated.md",
            "allowed_hashes": ["hash1"]
        }"#;
        let item: MigrationItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.type_, MigrationType::SafeFileDelete);
        assert_eq!(item.from, "deprecated.md");
        assert_eq!(
            item.allowed_hashes.as_deref(),
            Some(&["hash1".to_string()][..])
        );
    }

    #[test]
    fn test_deserialize_manifest() {
        let json = r###"{
            "version": "1.0.0",
            "description": "Release",
            "migrations": [
                {"type": "rename", "from": "a", "to": "b"},
                {"type": "delete", "from": "c"}
            ],
            "changelog": "## 1.0.0\n- Changes",
            "breaking": false,
            "recommendMigrate": false,
            "migrationGuide": "Do X then Y",
            "aiInstructions": "Help user with migration"
        }"###;
        let manifest: MigrationManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.description.as_deref(), Some("Release"));
        assert_eq!(manifest.migrations.len(), 2);
        assert_eq!(manifest.migrations[0].type_, MigrationType::Rename);
        assert_eq!(manifest.migrations[1].type_, MigrationType::Delete);
        assert_eq!(manifest.breaking, Some(false));
        assert_eq!(manifest.recommend_migrate, Some(false));
        assert!(manifest.changelog.is_some());
        assert!(manifest.migration_guide.is_some());
        assert!(manifest.ai_instructions.is_some());
    }

    #[test]
    fn test_migration_result_defaults() {
        let r = MigrationResult::default();
        assert_eq!(r.renamed, 0);
        assert_eq!(r.deleted, 0);
        assert_eq!(r.skipped, 0);
        assert_eq!(r.conflicts, 0);
    }
}
