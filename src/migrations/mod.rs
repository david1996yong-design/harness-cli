//! Migration system for Harness CLI updates.
//!
//! Migration manifests are JSON files embedded at compile time from
//! `embedded/manifests/`. Each manifest describes
//! file renames, deletions, and safe-file-deletes for a specific version.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

use rust_embed::Embed;

use crate::types::migration::{MigrationItem, MigrationManifest, MigrationType};
use crate::utils::compare_versions::compare_versions;

// ---------------------------------------------------------------------------
// Embedded manifests
// ---------------------------------------------------------------------------

#[derive(Embed)]
#[folder = "embedded/manifests/"]
struct MigrationManifests;

// ---------------------------------------------------------------------------
// Cached manifest store
// ---------------------------------------------------------------------------

/// Lazily-loaded and cached migration manifests.
static MANIFEST_CACHE: OnceLock<HashMap<String, MigrationManifest>> = OnceLock::new();

// ---------------------------------------------------------------------------
// Summary / Metadata types
// ---------------------------------------------------------------------------

/// Human-readable counts of pending migrations.
#[derive(Debug, Clone, Default)]
pub struct MigrationSummary {
    pub renames: usize,
    pub deletes: usize,
    pub safe_file_deletes: usize,
}

/// A single migration guide entry from a manifest.
#[derive(Debug, Clone)]
pub struct MigrationGuideEntry {
    pub version: String,
    pub guide: String,
    pub ai_instructions: Option<String>,
}

/// Aggregated metadata for migrations between two versions.
#[derive(Debug, Clone, Default)]
pub struct MigrationMetadata {
    pub changelog: Vec<String>,
    pub breaking: bool,
    pub recommend_migrate: bool,
    pub migration_guides: Vec<MigrationGuideEntry>,
}

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

/// Load all embedded JSON manifests into a `HashMap` keyed by version string.
pub fn load_manifests() -> &'static HashMap<String, MigrationManifest> {
    MANIFEST_CACHE.get_or_init(|| {
        let mut manifests = HashMap::new();

        for file_name in MigrationManifests::iter() {
            let name = file_name.as_ref();
            if !name.ends_with(".json") {
                continue;
            }

            let Some(asset) = MigrationManifests::get(name) else {
                continue;
            };

            let content = String::from_utf8_lossy(&asset.data);
            match serde_json::from_str::<MigrationManifest>(&content) {
                Ok(manifest) => {
                    let version = manifest.version.clone();
                    manifests.insert(version, manifest);
                }
                Err(_) => {
                    eprintln!("Warning: Failed to load migration manifest {}", name);
                }
            }
        }

        manifests
    })
}

/// Get all migrations needed to upgrade from `from` to `to`.
///
/// Returns migration items from versions that are strictly greater than `from`
/// and less than or equal to `to`, sorted in version order.
pub fn get_migrations_for_version(from: &str, to: &str) -> Vec<MigrationItem> {
    let manifests = load_manifests();

    let mut versions: Vec<&String> = manifests.keys().collect();
    versions.sort_by(|a, b| compare_versions(a, b));

    let mut all_migrations = Vec::new();

    for version in versions {
        let after_from = compare_versions(version, from) == Ordering::Greater;
        let at_or_before_to = compare_versions(version, to) != Ordering::Greater;

        if after_from && at_or_before_to {
            if let Some(manifest) = manifests.get(version.as_str()) {
                all_migrations.extend(manifest.migrations.iter().cloned());
            }
        }
    }

    all_migrations
}

/// Check if there are any pending migrations between versions.
pub fn has_pending_migrations(from: &str, to: &str) -> bool {
    !get_migrations_for_version(from, to).is_empty()
}

/// Get a human-readable summary of pending migrations.
pub fn get_migration_summary(from: &str, to: &str) -> MigrationSummary {
    let migrations = get_migrations_for_version(from, to);
    MigrationSummary {
        renames: migrations
            .iter()
            .filter(|m| matches!(m.type_, MigrationType::Rename))
            .count(),
        deletes: migrations
            .iter()
            .filter(|m| matches!(m.type_, MigrationType::Delete))
            .count(),
        safe_file_deletes: migrations
            .iter()
            .filter(|m| matches!(m.type_, MigrationType::SafeFileDelete))
            .count(),
    }
}

/// Get all registered migration versions, sorted.
pub fn get_all_migration_versions() -> Vec<String> {
    let manifests = load_manifests();
    let mut versions: Vec<String> = manifests.keys().cloned().collect();
    versions.sort_by(|a, b| compare_versions(a, b));
    versions
}

/// Get ALL migrations from all manifests (regardless of version).
///
/// Useful for detecting orphaned migrations that should have been applied.
pub fn get_all_migrations() -> Vec<MigrationItem> {
    let manifests = load_manifests();
    let mut all = Vec::new();
    for manifest in manifests.values() {
        all.extend(manifest.migrations.iter().cloned());
    }
    all
}

/// Get aggregated metadata for migrations between versions.
///
/// Returns combined changelog, breaking status, migrate recommendation,
/// and migration guides for all versions between `from` (exclusive)
/// and `to` (inclusive).
pub fn get_migration_metadata(from: &str, to: &str) -> MigrationMetadata {
    let manifests = load_manifests();

    let mut versions: Vec<&String> = manifests.keys().collect();
    versions.sort_by(|a, b| compare_versions(a, b));

    let mut result = MigrationMetadata::default();

    for version in versions {
        let after_from = compare_versions(version, from) == Ordering::Greater;
        let at_or_before_to = compare_versions(version, to) != Ordering::Greater;

        if after_from && at_or_before_to {
            if let Some(manifest) = manifests.get(version.as_str()) {
                if let Some(ref changelog) = manifest.changelog {
                    result
                        .changelog
                        .push(format!("v{}: {}", version, changelog));
                }
                if manifest.breaking == Some(true) {
                    result.breaking = true;
                }
                if manifest.recommend_migrate == Some(true) {
                    result.recommend_migrate = true;
                }
                if let Some(ref guide) = manifest.migration_guide {
                    result.migration_guides.push(MigrationGuideEntry {
                        version: version.clone(),
                        guide: guide.clone(),
                        ai_instructions: manifest.ai_instructions.clone(),
                    });
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_manifests_returns_non_empty() {
        let manifests = load_manifests();
        assert!(
            !manifests.is_empty(),
            "Should load at least one migration manifest"
        );
    }

    #[test]
    fn all_migration_versions_sorted() {
        let versions = get_all_migration_versions();
        for window in versions.windows(2) {
            assert!(
                compare_versions(&window[0], &window[1]) != Ordering::Greater,
                "Versions should be sorted: {} should come before {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn no_migrations_same_version() {
        assert!(!has_pending_migrations("0.3.0", "0.3.0"));
    }

    #[test]
    fn migration_summary_counts() {
        let summary = get_migration_summary("0.0.0", "99.99.99");
        // Just check it returns without panicking; actual counts depend on manifests
        let _ = summary.renames + summary.deletes + summary.safe_file_deletes;
    }

    #[test]
    fn migration_metadata_aggregates() {
        let metadata = get_migration_metadata("0.0.0", "99.99.99");
        // Should not panic and may have breaking changes
        let _ = metadata.breaking;
        let _ = metadata.recommend_migrate;
    }

    #[test]
    fn test_no_migrations_reversed() {
        // from > to should return no migrations
        let migrations = get_migrations_for_version("99.99.99", "0.0.0");
        assert!(
            migrations.is_empty(),
            "Reversed version range should return empty migrations"
        );
    }

    #[test]
    fn test_has_pending_same_version() {
        assert!(
            !has_pending_migrations("0.3.0", "0.3.0"),
            "Same version should have no pending migrations"
        );
    }

    #[test]
    fn test_migration_summary_same_version() {
        let summary = get_migration_summary("0.3.0", "0.3.0");
        assert_eq!(summary.renames, 0);
        assert_eq!(summary.deletes, 0);
        assert_eq!(summary.safe_file_deletes, 0);
    }

    #[test]
    fn test_migration_metadata_same_version() {
        let metadata = get_migration_metadata("0.3.0", "0.3.0");
        assert!(
            metadata.changelog.is_empty(),
            "Same version should have empty changelog"
        );
        assert!(
            !metadata.breaking,
            "Same version should not be breaking"
        );
    }

    #[test]
    fn test_all_migrations_have_from() {
        let all = get_all_migrations();
        for item in &all {
            assert!(
                !item.from.is_empty(),
                "Every migration item should have a non-empty 'from' field"
            );
        }
    }

    #[test]
    fn test_all_migrations_valid_type() {
        let all = get_all_migrations();
        for item in &all {
            // MigrationType is an enum, so if deserialization succeeded the type is valid.
            // We just verify it matches one of the known variants.
            let is_valid = matches!(
                item.type_,
                MigrationType::Rename
                    | MigrationType::RenameDir
                    | MigrationType::Delete
                    | MigrationType::SafeFileDelete
            );
            assert!(
                is_valid,
                "Migration item from '{}' should have a valid type",
                item.from
            );
        }
    }

    #[test]
    fn test_rename_migrations_have_to() {
        let all = get_all_migrations();
        for item in &all {
            if matches!(item.type_, MigrationType::Rename | MigrationType::RenameDir) {
                assert!(
                    item.to.is_some(),
                    "Rename/rename-dir migration from '{}' should have a 'to' field",
                    item.from
                );
            }
        }
    }

    #[test]
    fn test_safe_file_delete_have_hashes() {
        let all = get_all_migrations();
        for item in &all {
            if matches!(item.type_, MigrationType::SafeFileDelete) {
                assert!(
                    item.allowed_hashes.is_some(),
                    "safe-file-delete migration from '{}' should have allowed_hashes",
                    item.from
                );
                let hashes = item.allowed_hashes.as_ref().unwrap();
                assert!(
                    !hashes.is_empty(),
                    "safe-file-delete migration from '{}' should have non-empty allowed_hashes",
                    item.from
                );
            }
        }
    }
}
