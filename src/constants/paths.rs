//! Path constants for Harness CLI workflow structure
//!
//! Change these values to rename directories across the entire project.
//! All paths are relative to the project root.

// ---------------------------------------------------------------------------
// Directory names
// ---------------------------------------------------------------------------

/// Atomic directory name constants.
pub mod dir_names {
    /// Root workflow directory.
    pub const WORKFLOW: &str = ".harness-cli";
    /// Workspace directory (under `.harness-cli/`) -- developer work areas.
    pub const WORKSPACE: &str = "workspace";
    /// Tasks directory (under `.harness-cli/`) -- unified task storage.
    pub const TASKS: &str = "tasks";
    /// Archive directory (under `tasks/`).
    pub const ARCHIVE: &str = "archive";
    /// Spec/guidelines directory (under `.harness-cli/`).
    pub const SPEC: &str = "spec";
    /// Scripts directory (under `.harness-cli/`).
    pub const SCRIPTS: &str = "scripts";
}

// ---------------------------------------------------------------------------
// File names
// ---------------------------------------------------------------------------

/// Atomic file name constants.
pub mod file_names {
    /// Developer identity file.
    pub const DEVELOPER: &str = ".developer";
    /// Current task pointer.
    pub const CURRENT_TASK: &str = ".current-task";
    /// Task metadata.
    pub const TASK_JSON: &str = "task.json";
    /// Requirements document.
    pub const PRD: &str = "prd.md";
    /// Workflow guide.
    pub const WORKFLOW_GUIDE: &str = "workflow.md";
    /// Journal file prefix.
    pub const JOURNAL_PREFIX: &str = "journal-";
}

// ---------------------------------------------------------------------------
// Constructed paths (relative to project root)
// ---------------------------------------------------------------------------

/// Pre-constructed paths relative to the project root.
///
/// These combine [`dir_names`] and [`file_names`] into full relative paths.
pub mod constructed {
    /// `.harness-cli/`
    pub const WORKFLOW: &str = ".harness-cli";
    /// `.harness-cli/workspace/`
    pub const WORKSPACE: &str = ".harness-cli/workspace";
    /// `.harness-cli/tasks/`
    pub const TASKS: &str = ".harness-cli/tasks";
    /// `.harness-cli/spec/`
    pub const SPEC: &str = ".harness-cli/spec";
    /// `.harness-cli/scripts/`
    pub const SCRIPTS: &str = ".harness-cli/scripts";
    /// `.harness-cli/.developer`
    pub const DEVELOPER_FILE: &str = ".harness-cli/.developer";
    /// `.harness-cli/.current-task`
    pub const CURRENT_TASK_FILE: &str = ".harness-cli/.current-task";
    /// `.harness-cli/workflow.md`
    pub const WORKFLOW_GUIDE_FILE: &str = ".harness-cli/workflow.md";
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Get developer's workspace directory path.
///
/// # Example
///
/// ```
/// # use harness_cli::constants::paths::get_workspace_dir;
/// assert_eq!(get_workspace_dir("john"), ".harness-cli/workspace/john");
/// ```
pub fn get_workspace_dir(developer: &str) -> String {
    format!("{}/{}", constructed::WORKSPACE, developer)
}

/// Get task directory path.
///
/// # Example
///
/// ```
/// # use harness_cli::constants::paths::get_task_dir;
/// assert_eq!(get_task_dir("01-21-my-task"), ".harness-cli/tasks/01-21-my-task");
/// ```
pub fn get_task_dir(task_name: &str) -> String {
    format!("{}/{}", constructed::TASKS, task_name)
}

/// Get archive directory path.
///
/// # Example
///
/// ```
/// # use harness_cli::constants::paths::get_archive_dir;
/// assert_eq!(get_archive_dir(), ".harness-cli/tasks/archive");
/// ```
pub fn get_archive_dir() -> String {
    format!("{}/{}", constructed::TASKS, dir_names::ARCHIVE)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_dir() {
        assert_eq!(get_workspace_dir("alice"), ".harness-cli/workspace/alice");
    }

    #[test]
    fn task_dir() {
        assert_eq!(
            get_task_dir("04-10-feature"),
            ".harness-cli/tasks/04-10-feature"
        );
    }

    #[test]
    fn archive_dir() {
        assert_eq!(get_archive_dir(), ".harness-cli/tasks/archive");
    }

    #[test]
    fn constructed_paths_are_consistent() {
        // Verify that the constructed paths match what we would build from dir_names.
        assert_eq!(constructed::WORKFLOW, dir_names::WORKFLOW);
        assert_eq!(
            constructed::WORKSPACE,
            format!("{}/{}", dir_names::WORKFLOW, dir_names::WORKSPACE)
        );
        assert_eq!(
            constructed::TASKS,
            format!("{}/{}", dir_names::WORKFLOW, dir_names::TASKS)
        );
        assert_eq!(
            constructed::SPEC,
            format!("{}/{}", dir_names::WORKFLOW, dir_names::SPEC)
        );
        assert_eq!(
            constructed::SCRIPTS,
            format!("{}/{}", dir_names::WORKFLOW, dir_names::SCRIPTS)
        );
    }

    // --- Additional tests ported from TypeScript ---

    #[test]
    fn test_workflow_is_harness_cli() {
        assert_eq!(dir_names::WORKFLOW, ".harness-cli");
    }

    #[test]
    fn test_all_paths_start_with_workflow() {
        let paths = [
            constructed::WORKFLOW,
            constructed::WORKSPACE,
            constructed::TASKS,
            constructed::SPEC,
            constructed::SCRIPTS,
            constructed::DEVELOPER_FILE,
            constructed::CURRENT_TASK_FILE,
            constructed::WORKFLOW_GUIDE_FILE,
        ];
        for path in &paths {
            assert!(
                path.starts_with(".harness-cli"),
                "Path '{}' does not start with '.harness-cli'",
                path
            );
        }
    }

    #[test]
    fn test_workspace_path() {
        assert_eq!(constructed::WORKSPACE, ".harness-cli/workspace");
    }

    #[test]
    fn test_tasks_path() {
        assert_eq!(constructed::TASKS, ".harness-cli/tasks");
    }

    #[test]
    fn test_spec_path() {
        assert_eq!(constructed::SPEC, ".harness-cli/spec");
    }

    #[test]
    fn test_scripts_path() {
        assert_eq!(constructed::SCRIPTS, ".harness-cli/scripts");
    }

    #[test]
    fn test_developer_file() {
        assert_eq!(constructed::DEVELOPER_FILE, ".harness-cli/.developer");
    }

    #[test]
    fn test_current_task_file() {
        assert_eq!(
            constructed::CURRENT_TASK_FILE,
            ".harness-cli/.current-task"
        );
    }

    #[test]
    fn test_workspace_dir() {
        assert_eq!(get_workspace_dir("john"), ".harness-cli/workspace/john");
    }

    #[test]
    fn test_task_dir() {
        assert_eq!(get_task_dir("my-task"), ".harness-cli/tasks/my-task");
    }

    #[test]
    fn test_archive_dir_path() {
        assert_eq!(get_archive_dir(), ".harness-cli/tasks/archive");
    }

    #[test]
    fn test_uses_forward_slash() {
        let paths = [
            constructed::WORKFLOW,
            constructed::WORKSPACE,
            constructed::TASKS,
            constructed::SPEC,
            constructed::SCRIPTS,
            constructed::DEVELOPER_FILE,
            constructed::CURRENT_TASK_FILE,
            constructed::WORKFLOW_GUIDE_FILE,
        ];
        for path in &paths {
            assert!(
                !path.contains('\\'),
                "Path '{}' contains backslash",
                path
            );
        }
        // Also test dynamic paths.
        let ws = get_workspace_dir("test");
        assert!(!ws.contains('\\'), "Workspace dir '{}' contains backslash", ws);
        let td = get_task_dir("test");
        assert!(!td.contains('\\'), "Task dir '{}' contains backslash", td);
        let ad = get_archive_dir();
        assert!(!ad.contains('\\'), "Archive dir '{}' contains backslash", ad);
    }
}
