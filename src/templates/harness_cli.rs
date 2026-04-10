//! Harness CLI workflow templates.
//!
//! Provides access to core workflow files (`workflow.md`, `config.yaml`, etc.)
//! and the Python scripts that ship with every Harness CLI project.

use super::extract::{get_embedded_file, list_files, HarnessCliTemplates};

// ---------------------------------------------------------------------------
// Configuration file accessors
// ---------------------------------------------------------------------------

/// Get `config.yaml` content.
pub fn config_yaml_template() -> &'static str {
    static CONTENT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CONTENT
        .get_or_init(|| get_embedded_file::<HarnessCliTemplates>("config.yaml").unwrap_or_default())
}

/// Get `worktree.yaml` content.
pub fn worktree_yaml_template() -> &'static str {
    static CONTENT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CONTENT.get_or_init(|| {
        get_embedded_file::<HarnessCliTemplates>("worktree.yaml").unwrap_or_default()
    })
}

/// Get `.gitignore` content (stored as `gitignore.txt` in templates).
pub fn gitignore_template() -> &'static str {
    static CONTENT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CONTENT.get_or_init(|| {
        get_embedded_file::<HarnessCliTemplates>("gitignore.txt").unwrap_or_default()
    })
}

/// Get `workflow.md` content.
pub fn workflow_md_template() -> &'static str {
    static CONTENT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CONTENT
        .get_or_init(|| get_embedded_file::<HarnessCliTemplates>("workflow.md").unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Scripts
// ---------------------------------------------------------------------------

/// Get all Python scripts from `scripts/` as `(relative_path, content)` pairs.
///
/// The relative path is relative to `scripts/` (e.g. `common/paths.py`,
/// `task.py`, `multi_agent/start.py`).
pub fn get_all_scripts() -> Vec<(String, String)> {
    let mut scripts = Vec::new();
    for path in list_files::<HarnessCliTemplates>() {
        if path.starts_with("scripts/") {
            if let Some(content) = get_embedded_file::<HarnessCliTemplates>(&path) {
                let rel = path.strip_prefix("scripts/").unwrap().to_string();
                scripts.push((rel, content));
            }
        }
    }
    scripts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_yaml_non_empty() {
        let content = config_yaml_template();
        assert!(
            !content.is_empty(),
            "config.yaml template should be non-empty"
        );
    }

    #[test]
    fn test_worktree_yaml_non_empty() {
        let content = worktree_yaml_template();
        assert!(
            !content.is_empty(),
            "worktree.yaml template should be non-empty"
        );
    }

    #[test]
    fn test_gitignore_non_empty() {
        let content = gitignore_template();
        assert!(
            !content.is_empty(),
            "gitignore template should be non-empty"
        );
    }

    #[test]
    fn test_workflow_md_non_empty() {
        let content = workflow_md_template();
        assert!(
            !content.is_empty(),
            "workflow.md template should be non-empty"
        );
    }

    #[test]
    fn test_get_all_scripts_non_empty() {
        let scripts = get_all_scripts();
        assert!(!scripts.is_empty(), "Scripts map should be non-empty");
    }

    #[test]
    fn test_scripts_contain_python() {
        for (key, content) in get_all_scripts() {
            // Skip __init__.py files which can be empty marker files
            if key.ends_with("__init__.py") {
                continue;
            }
            if key.ends_with(".py") {
                let has_python = content.contains("def ")
                    || content.contains("import ")
                    || content.contains("from ");
                assert!(
                    has_python,
                    "Script '{}' should contain Python syntax (def, import, or from)",
                    key
                );
            }
        }
    }

    #[test]
    fn test_scripts_keys_end_with_py() {
        for (key, _) in get_all_scripts() {
            assert!(
                key.ends_with(".py"),
                "Script key '{}' should end with .py",
                key
            );
        }
    }
}
