use std::collections::HashMap;
use std::path::Path;

use sha2::{Digest, Sha256};

/// File name for storing template hashes.
const HASHES_FILE: &str = ".template-hashes.json";

/// Patterns to exclude from hash tracking.
const EXCLUDE_FROM_HASH: &[&str] = &[
    ".template-hashes.json",
    ".version",
    ".gitignore",
    ".developer",
    "workspace/",
    "tasks/",
    ".current-task",
    "spec/",
    ".backup-",
];

// =============================================================================
// Public API
// =============================================================================

/// Compute the SHA256 hash of content and return it as a hex string.
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Get the path to the hashes file.
fn get_hashes_path(cwd: &Path) -> std::path::PathBuf {
    cwd.join(".harness-cli").join(HASHES_FILE)
}

/// Load stored template hashes from `.harness-cli/.template-hashes.json`.
pub fn load_hashes(cwd: &Path) -> HashMap<String, String> {
    let hashes_path = get_hashes_path(cwd);
    if !hashes_path.exists() {
        return HashMap::new();
    }

    match std::fs::read_to_string(&hashes_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Save template hashes to `.harness-cli/.template-hashes.json`.
pub fn save_hashes(cwd: &Path, hashes: &HashMap<String, String>) {
    let hashes_path = get_hashes_path(cwd);

    // Ensure parent directory exists.
    if let Some(parent) = hashes_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(hashes) {
        let _ = std::fs::write(&hashes_path, json);
    }
}

/// Update hashes for specific files.
///
/// `files` is a map of relative paths to file contents.
pub fn update_hashes(cwd: &Path, files: &HashMap<String, String>) {
    let mut hashes = load_hashes(cwd);

    for (relative_path, content) in files {
        hashes.insert(relative_path.clone(), compute_hash(content));
    }

    save_hashes(cwd, &hashes);
}

/// Update the hash for a single file by reading its current content from disk.
pub fn update_hash_from_file(cwd: &Path, relative_path: &str) {
    let full_path = cwd.join(relative_path);
    if !full_path.exists() {
        return;
    }

    if let Ok(content) = std::fs::read_to_string(&full_path) {
        let mut hashes = load_hashes(cwd);
        hashes.insert(relative_path.to_string(), compute_hash(&content));
        save_hashes(cwd, &hashes);
    }
}

/// Remove a hash entry for a file (e.g., after deletion).
pub fn remove_hash(cwd: &Path, relative_path: &str) {
    let mut hashes = load_hashes(cwd);
    hashes.remove(relative_path);
    save_hashes(cwd, &hashes);
}

/// Rename a hash entry (used after a file rename).
pub fn rename_hash(cwd: &Path, old_path: &str, new_path: &str) {
    let mut hashes = load_hashes(cwd);
    if let Some(value) = hashes.remove(old_path) {
        hashes.insert(new_path.to_string(), value);
        save_hashes(cwd, &hashes);
    }
}

/// Check if a template file has been modified by the user.
///
/// Returns `true` if the file's current content differs from the stored hash,
/// or if there is no stored hash (conservative assumption).
pub fn is_template_modified(
    cwd: &Path,
    relative_path: &str,
    hashes: &HashMap<String, String>,
) -> bool {
    let full_path = cwd.join(relative_path);

    // If file doesn't exist, it can't be modified.
    if !full_path.exists() {
        return false;
    }

    // If we don't have a stored hash, assume it's modified (conservative).
    let stored_hash = match hashes.get(relative_path) {
        Some(h) => h,
        None => return true,
    };

    // Compare current content hash with stored hash.
    match std::fs::read_to_string(&full_path) {
        Ok(content) => compute_hash(&content) != *stored_hash,
        Err(_) => true,
    }
}

/// Check if a path should be excluded from hash tracking.
fn should_exclude_from_hash(relative_path: &str) -> bool {
    EXCLUDE_FROM_HASH
        .iter()
        .any(|pattern| relative_path.contains(pattern))
}

/// Recursively collect all files in a directory, excluding patterns.
fn collect_files(cwd: &Path, dir: &str) -> Vec<String> {
    let full_dir = cwd.join(dir);
    if !full_dir.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();

    let entries = match std::fs::read_dir(&full_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let relative_path = if dir.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", dir, name)
        };

        if should_exclude_from_hash(&relative_path) {
            continue;
        }

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            files.extend(collect_files(cwd, &relative_path));
        } else if file_type.is_file() {
            files.push(relative_path);
        }
    }

    files
}

/// Initialize template hashes after init.
///
/// Scans all `managed_dirs`, computes hashes for their files, and saves them.
/// Returns the number of files hashed.
pub fn initialize_hashes(cwd: &Path, managed_dirs: &[&str]) -> usize {
    let mut hashes: HashMap<String, String> = HashMap::new();

    for dir in managed_dirs {
        let files = collect_files(cwd, dir);

        for relative_path in files {
            let full_path = cwd.join(&relative_path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                hashes.insert(relative_path, compute_hash(&content));
            }
        }
    }

    let count = hashes.len();
    save_hashes(cwd, &hashes);
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_should_exclude() {
        assert!(should_exclude_from_hash(".template-hashes.json"));
        assert!(should_exclude_from_hash("some/path/.version"));
        assert!(should_exclude_from_hash("tasks/foo.md"));
        assert!(should_exclude_from_hash("workspace/data.json"));
        assert!(should_exclude_from_hash(".backup-2024"));
        assert!(!should_exclude_from_hash("src/main.rs"));
    }

    // --- Additional tests ported from TypeScript ---

    #[test]
    fn test_compute_hash_known() {
        // SHA256 of "hello"
        assert_eq!(
            compute_hash("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_compute_hash_consistent() {
        let h1 = compute_hash("same input");
        let h2 = compute_hash("same input");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_hash_different() {
        let h1 = compute_hash("input a");
        let h2 = compute_hash("input b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_compute_hash_empty() {
        let h = compute_hash("");
        assert!(!h.is_empty());
        // SHA256 of empty string is a well-known value.
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_load_save_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut data = HashMap::new();
        data.insert("file.txt".to_string(), "hash123".to_string());
        data.insert("dir/other.md".to_string(), "hash456".to_string());

        save_hashes(tmp.path(), &data);
        let loaded = load_hashes(tmp.path());
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("file.txt").unwrap(), "hash123");
        assert_eq!(loaded.get("dir/other.md").unwrap(), "hash456");
    }

    #[test]
    fn test_load_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = load_hashes(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let hashes_dir = tmp.path().join(".harness-cli");
        std::fs::create_dir_all(&hashes_dir).unwrap();
        std::fs::write(
            hashes_dir.join(".template-hashes.json"),
            "this is not json{{{",
        )
        .unwrap();
        let loaded = load_hashes(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_update_hashes_adds_entries() {
        let tmp = tempfile::tempdir().unwrap();
        // Save initial entry.
        let mut initial = HashMap::new();
        initial.insert("existing.txt".to_string(), "aaa".to_string());
        save_hashes(tmp.path(), &initial);

        // Add new entries via update_hashes.
        let mut new_files = HashMap::new();
        new_files.insert("new.txt".to_string(), "content".to_string());
        update_hashes(tmp.path(), &new_files);

        let loaded = load_hashes(tmp.path());
        // Existing entry should still be there.
        assert_eq!(loaded.get("existing.txt").unwrap(), "aaa");
        // New entry should have been added (with computed hash of "content").
        assert!(loaded.contains_key("new.txt"));
        assert_eq!(loaded["new.txt"], compute_hash("content"));
    }

    #[test]
    fn test_update_hash_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("hello.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        update_hash_from_file(tmp.path(), "hello.txt");

        let loaded = load_hashes(tmp.path());
        let expected = compute_hash("hello world");
        assert_eq!(loaded.get("hello.txt").unwrap(), &expected);
    }

    #[test]
    fn test_remove_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let mut data = HashMap::new();
        data.insert("keep.txt".to_string(), "h1".to_string());
        data.insert("remove.txt".to_string(), "h2".to_string());
        save_hashes(tmp.path(), &data);

        remove_hash(tmp.path(), "remove.txt");

        let loaded = load_hashes(tmp.path());
        assert!(loaded.contains_key("keep.txt"));
        assert!(!loaded.contains_key("remove.txt"));
    }

    #[test]
    fn test_rename_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let mut data = HashMap::new();
        data.insert("old.txt".to_string(), "the_hash".to_string());
        save_hashes(tmp.path(), &data);

        rename_hash(tmp.path(), "old.txt", "new.txt");

        let loaded = load_hashes(tmp.path());
        assert!(!loaded.contains_key("old.txt"));
        assert_eq!(loaded.get("new.txt").unwrap(), "the_hash");
    }

    #[test]
    fn test_is_template_modified_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let hashes = HashMap::new();
        // File doesn't exist -> not modified.
        assert!(!is_template_modified(
            tmp.path(),
            "nonexistent.txt",
            &hashes
        ));
    }

    #[test]
    fn test_is_template_modified_no_hash() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        let hashes = HashMap::new();
        // No stored hash -> conservative assumption = modified.
        assert!(is_template_modified(tmp.path(), "file.txt", &hashes));
    }

    #[test]
    fn test_is_template_modified_matching() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        let mut hashes = HashMap::new();
        hashes.insert("file.txt".to_string(), compute_hash("content"));
        // Matching hash -> not modified.
        assert!(!is_template_modified(tmp.path(), "file.txt", &hashes));
    }

    #[test]
    fn test_is_template_modified_changed() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "new content").unwrap();
        let mut hashes = HashMap::new();
        hashes.insert("file.txt".to_string(), compute_hash("old content"));
        // Different hash -> modified.
        assert!(is_template_modified(tmp.path(), "file.txt", &hashes));
    }

    #[test]
    fn test_initialize_hashes_empty() {
        let tmp = tempfile::tempdir().unwrap();
        // No dirs to scan -> 0 files hashed.
        let count = initialize_hashes(tmp.path(), &[]);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_initialize_hashes_excludes() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("test_dir");
        std::fs::create_dir_all(&base).unwrap();

        // Create a regular file (should be hashed).
        std::fs::write(base.join("readme.md"), "hello").unwrap();

        // Create excluded dirs and files.
        let ws = base.join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("data.txt"), "ws data").unwrap();

        let tasks = base.join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(tasks.join("task1.json"), "{}").unwrap();

        let spec = base.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(spec.join("guide.md"), "spec data").unwrap();

        let count = initialize_hashes(tmp.path(), &["test_dir"]);
        // Only readme.md should be hashed (workspace/, tasks/, spec/ are excluded).
        assert_eq!(count, 1);

        let loaded = load_hashes(tmp.path());
        assert!(loaded.contains_key("test_dir/readme.md"));
        assert!(!loaded.contains_key("test_dir/workspace/data.txt"));
        assert!(!loaded.contains_key("test_dir/tasks/task1.json"));
        assert!(!loaded.contains_key("test_dir/spec/guide.md"));
    }
}
