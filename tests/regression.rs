//! Regression and integration tests for harness-cli.
//!
//! Ported from the TypeScript regression suite (~130 tests).  Tests are
//! organised by category and exercise the library crate's public API without
//! spawning the binary.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use harness_cli::configurators;
use harness_cli::configurators::shared::resolve_placeholders;
use harness_cli::configurators::workflow::{create_workflow_structure, WorkflowOptions};
use harness_cli::constants::paths::{
    constructed, dir_names, get_archive_dir, get_task_dir, get_workspace_dir,
};
use harness_cli::constants::version::VERSION;
use harness_cli::migrations;
use harness_cli::templates::extract::{list_files, HarnessCliTemplates};
use harness_cli::templates::{
    claude, codex, copilot, harness_cli as hc_templates, iflow, markdown,
};
use harness_cli::types::ai_tools::{get_managed_paths, get_tool_config, AITool};
use harness_cli::types::migration::MigrationType;
use harness_cli::utils::compare_versions::compare_versions;
use harness_cli::utils::file_writer::{ensure_dir, set_write_mode, write_file, WriteMode};
use harness_cli::utils::project_detector::ProjectType;
use harness_cli::utils::template_hash::{
    compute_hash, initialize_hashes, is_template_modified, load_hashes, save_hashes, update_hashes,
};

// =========================================================================
// 1. Platform Registration (13 + 1 tests)
// =========================================================================

#[test]
fn test_claude_code_registered() {
    let cfg = get_tool_config(AITool::ClaudeCode);
    assert_eq!(cfg.name, "Claude Code");
    assert_eq!(cfg.config_dir, ".claude");
}

#[test]
fn test_cursor_registered() {
    let cfg = get_tool_config(AITool::Cursor);
    assert_eq!(cfg.name, "Cursor");
    assert_eq!(cfg.config_dir, ".cursor");
}

#[test]
fn test_opencode_registered() {
    let cfg = get_tool_config(AITool::OpenCode);
    assert_eq!(cfg.name, "OpenCode");
    assert_eq!(cfg.config_dir, ".opencode");
}

#[test]
fn test_iflow_registered() {
    let cfg = get_tool_config(AITool::IFlow);
    assert_eq!(cfg.name, "iFlow CLI");
    assert_eq!(cfg.config_dir, ".iflow");
}

#[test]
fn test_codex_registered() {
    let cfg = get_tool_config(AITool::Codex);
    assert_eq!(cfg.name, "Codex");
    assert_eq!(cfg.config_dir, ".codex");
}

#[test]
fn test_kilo_registered() {
    let cfg = get_tool_config(AITool::Kilo);
    assert_eq!(cfg.name, "Kilo CLI");
    assert_eq!(cfg.config_dir, ".kilocode");
}

#[test]
fn test_kiro_registered() {
    let cfg = get_tool_config(AITool::Kiro);
    assert_eq!(cfg.name, "Kiro Code");
    assert_eq!(cfg.config_dir, ".kiro/skills");
}

#[test]
fn test_gemini_registered() {
    let cfg = get_tool_config(AITool::Gemini);
    assert_eq!(cfg.name, "Gemini CLI");
    assert_eq!(cfg.config_dir, ".gemini");
}

#[test]
fn test_antigravity_registered() {
    let cfg = get_tool_config(AITool::Antigravity);
    assert_eq!(cfg.name, "Antigravity");
    assert_eq!(cfg.config_dir, ".agent/workflows");
}

#[test]
fn test_windsurf_registered() {
    let cfg = get_tool_config(AITool::Windsurf);
    assert_eq!(cfg.name, "Windsurf");
    assert_eq!(cfg.config_dir, ".windsurf/workflows");
}

#[test]
fn test_qoder_registered() {
    let cfg = get_tool_config(AITool::Qoder);
    assert_eq!(cfg.name, "Qoder");
    assert_eq!(cfg.config_dir, ".qoder");
}

#[test]
fn test_codebuddy_registered() {
    let cfg = get_tool_config(AITool::CodeBuddy);
    assert_eq!(cfg.name, "CodeBuddy");
    assert_eq!(cfg.config_dir, ".codebuddy");
}

#[test]
fn test_copilot_registered() {
    let cfg = get_tool_config(AITool::Copilot);
    assert_eq!(cfg.name, "GitHub Copilot");
    assert_eq!(cfg.config_dir, ".github/copilot");
}

#[test]
fn test_all_platforms_have_consistent_fields() {
    for tool in AITool::all() {
        let cfg = get_tool_config(*tool);
        assert!(
            !cfg.name.is_empty(),
            "Platform {:?} has an empty name",
            tool
        );
        assert!(
            cfg.config_dir.starts_with('.'),
            "Platform {:?} config_dir '{}' does not start with '.'",
            tool,
            cfg.config_dir
        );
    }
}

// =========================================================================
// 2. Shell to Python Migration (4 tests)
// =========================================================================

#[test]
fn test_no_sh_scripts_in_templates() {
    let files = list_files::<HarnessCliTemplates>();
    let sh_files: Vec<_> = files
        .iter()
        .filter(|f| f.starts_with("scripts/") && f.ends_with(".sh"))
        .collect();
    assert!(
        sh_files.is_empty(),
        "Found .sh files in harness-cli scripts: {:?}",
        sh_files
    );
}

#[test]
fn test_all_script_keys_end_with_py() {
    let scripts = hc_templates::get_all_scripts();
    assert!(
        !scripts.is_empty(),
        "get_all_scripts should return at least one script"
    );
    for (key, _) in &scripts {
        assert!(
            key.ends_with(".py"),
            "Script key '{}' does not end with .py",
            key
        );
    }
}

#[test]
fn test_multi_agent_uses_underscore() {
    let scripts = hc_templates::get_all_scripts();
    let multi_agent_scripts: Vec<_> = scripts
        .iter()
        .filter(|(k, _)| k.contains("multi"))
        .collect();
    assert!(
        !multi_agent_scripts.is_empty(),
        "Should have multi_agent scripts"
    );
    for (key, _) in &multi_agent_scripts {
        assert!(
            key.contains("multi_agent"),
            "Expected underscore in '{}', not hyphen",
            key
        );
        assert!(
            !key.contains("multi-agent"),
            "Found hyphenated 'multi-agent' in key '{}'",
            key
        );
    }
}

#[test]
fn test_get_all_scripts_covers_all_py_files() {
    let all_files = list_files::<HarnessCliTemplates>();
    let py_files_in_scripts: HashSet<String> = all_files
        .iter()
        .filter(|f| f.starts_with("scripts/") && f.ends_with(".py"))
        .map(|f| f.strip_prefix("scripts/").unwrap().to_string())
        .collect();

    let scripts = hc_templates::get_all_scripts();
    let script_keys: HashSet<String> = scripts.into_iter().map(|(k, _)| k).collect();

    for py in &py_files_in_scripts {
        assert!(
            script_keys.contains(py),
            "Python file '{}' is not covered by get_all_scripts()",
            py
        );
    }
}

// =========================================================================
// 3. Hook JSON Format (5 tests)
// =========================================================================

#[test]
fn test_claude_settings_valid_json() {
    let settings = claude::get_settings_template();
    assert!(
        !settings.content.is_empty(),
        "Claude settings.json should not be empty"
    );
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&settings.content);
    assert!(
        parsed.is_ok(),
        "Claude settings.json is not valid JSON: {:?}",
        parsed.err()
    );
}

#[test]
fn test_claude_settings_has_hooks() {
    let settings = claude::get_settings_template();
    let parsed: serde_json::Value =
        serde_json::from_str(&settings.content).expect("settings.json should be valid JSON");
    assert!(
        parsed.get("hooks").is_some(),
        "Claude settings.json should have a 'hooks' key"
    );
}

#[test]
fn test_claude_hooks_use_python_cmd_placeholder() {
    let settings = claude::get_settings_template();
    assert!(
        settings.content.contains("{{PYTHON_CMD}}"),
        "Claude settings.json should contain {{{{PYTHON_CMD}}}} placeholder"
    );
}

#[test]
fn test_iflow_settings_valid_json() {
    let settings = iflow::get_settings_template();
    assert!(
        !settings.content.is_empty(),
        "iFlow settings.json should not be empty"
    );
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&settings.content);
    assert!(
        parsed.is_ok(),
        "iFlow settings.json is not valid JSON: {:?}",
        parsed.err()
    );
}

#[test]
fn test_iflow_hooks_use_python_cmd_placeholder() {
    let settings = iflow::get_settings_template();
    assert!(
        settings.content.contains("{{PYTHON_CMD}}"),
        "iFlow settings.json should contain {{{{PYTHON_CMD}}}} placeholder"
    );
}

// =========================================================================
// 4. Semver Prerelease Handling (4 tests)
// =========================================================================

#[test]
fn test_prerelease_before_release() {
    assert_eq!(
        compare_versions("0.3.0-beta.1", "0.3.0"),
        Ordering::Less,
        "0.3.0-beta.1 should be less than 0.3.0"
    );
}

#[test]
fn test_prerelease_numeric_parts() {
    assert_eq!(
        compare_versions("0.3.0-beta.2", "0.3.0-beta.10"),
        Ordering::Less,
        "beta.2 should be less than beta.10 (numeric comparison)"
    );
}

#[test]
fn test_no_migrations_equal_versions() {
    assert!(
        !migrations::has_pending_migrations("0.3.0", "0.3.0"),
        "Same version should have no pending migrations"
    );
}

#[test]
fn test_migrations_beta_range_works() {
    // Getting migrations between two beta versions should not panic
    let items = migrations::get_migrations_for_version("0.3.0-beta.1", "0.3.0-beta.5");
    // Just verify it doesn't panic and returns a Vec
    let _ = items.len();
}

// =========================================================================
// 5. Migration Data Integrity (5 tests)
// =========================================================================

#[test]
fn test_all_migrations_have_from() {
    let all = migrations::get_all_migrations();
    for item in &all {
        assert!(
            !item.from.is_empty(),
            "Migration item should have a non-empty 'from' field: {:?}",
            item
        );
    }
}

#[test]
fn test_all_migrations_valid_type() {
    let all = migrations::get_all_migrations();
    for item in &all {
        // This test verifies that deserialization worked for all items.
        // If a type were invalid, serde would have failed during load_manifests().
        match item.type_ {
            MigrationType::Rename
            | MigrationType::RenameDir
            | MigrationType::Delete
            | MigrationType::SafeFileDelete => {}
        }
    }
}

#[test]
fn test_safe_file_delete_have_hashes() {
    let all = migrations::get_all_migrations();
    let safe_deletes: Vec<_> = all
        .iter()
        .filter(|m| m.type_ == MigrationType::SafeFileDelete)
        .collect();
    for item in &safe_deletes {
        assert!(
            item.allowed_hashes.is_some() && !item.allowed_hashes.as_ref().unwrap().is_empty(),
            "SafeFileDelete for '{}' should have allowed_hashes",
            item.from
        );
    }
}

#[test]
fn test_rename_migrations_have_to() {
    let all = migrations::get_all_migrations();
    let renames: Vec<_> = all
        .iter()
        .filter(|m| m.type_ == MigrationType::Rename || m.type_ == MigrationType::RenameDir)
        .collect();
    for item in &renames {
        assert!(
            item.to.is_some() && !item.to.as_ref().unwrap().is_empty(),
            "Rename migration from '{}' should have a non-empty 'to' field",
            item.from
        );
    }
}

#[test]
fn test_all_manifest_versions_valid() {
    let versions = migrations::get_all_migration_versions();
    assert!(
        !versions.is_empty(),
        "Should have at least one migration manifest"
    );
    // All versions should be parseable (no panics) and sorted
    for window in versions.windows(2) {
        assert!(
            compare_versions(&window[0], &window[1]) != Ordering::Greater,
            "Versions out of order: {} > {}",
            window[0],
            window[1]
        );
    }
}

// =========================================================================
// 6. collectTemplates Path Consistency (4 tests)
// =========================================================================

#[test]
fn test_claude_collect_templates_paths() {
    let templates = configurators::collect_platform_templates(AITool::ClaudeCode)
        .expect("Claude should support template collection");
    assert!(
        !templates.is_empty(),
        "Claude templates should not be empty"
    );
    for path in templates.keys() {
        assert!(
            path.starts_with(".claude/"),
            "Claude template path '{}' should start with '.claude/'",
            path
        );
    }
}

#[test]
fn test_iflow_collect_templates_paths() {
    let templates = configurators::collect_platform_templates(AITool::IFlow)
        .expect("iFlow should support template collection");
    assert!(!templates.is_empty(), "iFlow templates should not be empty");
    for path in templates.keys() {
        assert!(
            path.starts_with(".iflow/"),
            "iFlow template path '{}' should start with '.iflow/'",
            path
        );
    }
}

#[test]
fn test_codex_collect_templates_agents_and_skills() {
    let templates = configurators::collect_platform_templates(AITool::Codex)
        .expect("Codex should support template collection");
    assert!(!templates.is_empty(), "Codex templates should not be empty");

    let has_agents = templates.keys().any(|p| p.starts_with(".agents/"));
    let has_codex = templates.keys().any(|p| p.starts_with(".codex/"));

    assert!(has_agents, "Codex templates should include .agents/ paths");
    assert!(has_codex, "Codex templates should include .codex/ paths");
}

#[test]
fn test_copilot_collect_templates_hooks() {
    let templates = configurators::collect_platform_templates(AITool::Copilot)
        .expect("Copilot should support template collection");
    assert!(
        !templates.is_empty(),
        "Copilot templates should not be empty"
    );

    let has_github = templates.keys().any(|p| p.starts_with(".github/"));
    assert!(
        has_github,
        "Copilot templates should include .github/ paths"
    );
}

// =========================================================================
// 7. Template Content Checks (4 tests)
// =========================================================================

#[test]
fn test_claude_command_names_match_expected_set() {
    let commands = claude::get_all_commands();
    let names: HashSet<String> = commands.iter().map(|c| c.name.clone()).collect();

    let expected = [
        "before-dev",
        "brainstorm",
        "break-loop",
        "check",
        "check-cross-layer",
        "create-command",
        "finish-work",
        "integrate-skill",
        "onboard",
        "parallel",
        "record-session",
        "start",
        "update-spec",
    ];

    for name in &expected {
        assert!(
            names.contains(*name),
            "Expected Claude command '{}' not found. Available: {:?}",
            name,
            names
        );
    }
}

#[test]
fn test_codex_skills_match_expected_set() {
    let skills = codex::get_all_skills();
    let names: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();

    let expected = [
        "before-dev",
        "brainstorm",
        "break-loop",
        "check",
        "check-cross-layer",
        "create-command",
        "finish-work",
        "improve-ut",
        "integrate-skill",
        "onboard",
        "record-session",
        "start",
        "update-spec",
    ];

    for name in &expected {
        assert!(
            names.contains(*name),
            "Expected Codex shared skill '{}' not found. Available: {:?}",
            name,
            names
        );
    }
}

#[test]
fn test_harness_cli_scripts_are_python() {
    let scripts = hc_templates::get_all_scripts();
    assert!(!scripts.is_empty(), "Should have harness-cli scripts");
    for (key, content) in &scripts {
        // __init__.py files may be empty, skip those
        if key.ends_with("__init__.py") {
            continue;
        }
        assert!(
            content.contains("def ") || content.contains("import "),
            "Script '{}' does not look like Python (missing 'def ' and 'import ')",
            key
        );
    }
}

#[test]
fn test_markdown_templates_all_non_empty() {
    // Test a representative set of markdown templates
    let templates: Vec<(&str, &str)> = vec![
        ("backend_index", markdown::backend_index_content()),
        (
            "backend_directory_structure",
            markdown::backend_directory_structure_content(),
        ),
        ("frontend_index", markdown::frontend_index_content()),
        (
            "frontend_directory_structure",
            markdown::frontend_directory_structure_content(),
        ),
        ("guides_index", markdown::guides_index_content()),
        (
            "guides_cross_layer",
            markdown::guides_cross_layer_thinking_guide_content(),
        ),
        (
            "guides_code_reuse",
            markdown::guides_code_reuse_thinking_guide_content(),
        ),
    ];

    for (name, content) in &templates {
        assert!(
            !content.is_empty(),
            "Markdown template '{}' should not be empty",
            name
        );
    }
}

// =========================================================================
// 8. Constants Consistency (3 tests)
// =========================================================================

#[test]
fn test_tasks_path_correct() {
    assert_eq!(
        constructed::TASKS,
        ".harness-cli/tasks",
        "Tasks path should be '.harness-cli/tasks'"
    );
}

#[test]
fn test_workspace_path_correct() {
    assert_eq!(
        constructed::WORKSPACE,
        ".harness-cli/workspace",
        "Workspace path should be '.harness-cli/workspace'"
    );
}

#[test]
fn test_developer_file_correct() {
    assert_eq!(
        constructed::DEVELOPER_FILE,
        ".harness-cli/.developer",
        "Developer file path should be '.harness-cli/.developer'"
    );
}

// =========================================================================
// 9. Placeholder Resolution (3 tests)
// =========================================================================

#[test]
fn test_resolve_placeholders_replaces_python_cmd() {
    let input = "run {{PYTHON_CMD}} script.py";
    let output = resolve_placeholders(input);
    assert!(
        !output.contains("{{PYTHON_CMD}}"),
        "Placeholder should be resolved"
    );
    // On Unix: python3, on Windows: python
    if cfg!(windows) {
        assert!(
            output.contains("python"),
            "Should contain 'python' on Windows"
        );
    } else {
        assert!(
            output.contains("python3"),
            "Should contain 'python3' on Unix"
        );
    }
}

#[test]
fn test_resolve_placeholders_preserves_other_content() {
    let input = "keep this text intact";
    let output = resolve_placeholders(input);
    assert_eq!(
        output, input,
        "Content without placeholders should be preserved"
    );
}

#[test]
fn test_resolve_placeholders_handles_multiple_occurrences() {
    let input = "{{PYTHON_CMD}} && {{PYTHON_CMD}}";
    let output = resolve_placeholders(input);
    let count = output.matches("{{PYTHON_CMD}}").count();
    assert_eq!(
        count, 0,
        "All occurrences of {{{{PYTHON_CMD}}}} should be resolved"
    );
    // Should have two occurrences of the resolved command
    let cmd = if cfg!(windows) { "python" } else { "python3" };
    let cmd_count = output.matches(cmd).count();
    assert_eq!(cmd_count, 2, "Should have two resolved python commands");
}

// =========================================================================
// 10. is_managed_path edge cases (5 tests)
// =========================================================================

#[test]
fn test_is_managed_path_claude_subpath() {
    assert!(
        configurators::is_managed_path(".claude/commands/hc/start.md"),
        ".claude/commands/hc/start.md should be managed"
    );
}

#[test]
fn test_is_managed_path_exact_match() {
    assert!(
        configurators::is_managed_path(".harness-cli"),
        ".harness-cli should be managed (exact match)"
    );
}

#[test]
fn test_is_managed_path_rejects_prefix_collision() {
    assert!(
        !configurators::is_managed_path(".claude-extra/foo"),
        ".claude-extra/foo should NOT be managed (prefix collision)"
    );
}

#[test]
fn test_is_managed_path_rejects_empty() {
    assert!(
        !configurators::is_managed_path(""),
        "Empty string should not be managed"
    );
}

#[test]
fn test_is_managed_path_windows_backslash() {
    assert!(
        configurators::is_managed_path(".claude\\commands\\start.md"),
        "Windows-style path should be normalized and matched"
    );
}

// =========================================================================
// 11. Integration-style tests with temp directories
// =========================================================================

#[test]
fn test_init_creates_directory_structure() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions {
        project_type: ProjectType::Fullstack,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    assert!(
        cwd.join(".harness-cli").is_dir(),
        ".harness-cli/ should exist"
    );
    assert!(
        cwd.join(".harness-cli/scripts").is_dir(),
        "scripts/ should exist"
    );
    assert!(
        cwd.join(".harness-cli/workspace").is_dir(),
        "workspace/ should exist"
    );
    assert!(
        cwd.join(".harness-cli/tasks").is_dir(),
        "tasks/ should exist"
    );
    assert!(cwd.join(".harness-cli/spec").is_dir(), "spec/ should exist");
    assert!(
        cwd.join(".harness-cli/spec/backend").is_dir(),
        "spec/backend/ should exist for Fullstack"
    );
    assert!(
        cwd.join(".harness-cli/spec/frontend").is_dir(),
        "spec/frontend/ should exist for Fullstack"
    );
    assert!(
        cwd.join(".harness-cli/spec/guides").is_dir(),
        "spec/guides/ should exist"
    );
}

#[test]
fn test_init_backend_skips_frontend_spec() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions {
        project_type: ProjectType::Backend,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    assert!(
        cwd.join(".harness-cli/spec/backend").is_dir(),
        "spec/backend/ should exist for Backend project"
    );
    assert!(
        !cwd.join(".harness-cli/spec/frontend").exists(),
        "spec/frontend/ should NOT exist for Backend project"
    );
}

#[test]
fn test_init_frontend_skips_backend_spec() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions {
        project_type: ProjectType::Frontend,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    assert!(
        cwd.join(".harness-cli/spec/frontend").is_dir(),
        "spec/frontend/ should exist for Frontend project"
    );
    assert!(
        !cwd.join(".harness-cli/spec/backend").exists(),
        "spec/backend/ should NOT exist for Frontend project"
    );
}

#[test]
fn test_init_creates_workflow_md() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions::default();
    create_workflow_structure(cwd, &options).unwrap();

    let workflow_path = cwd.join(".harness-cli/workflow.md");
    assert!(workflow_path.is_file(), "workflow.md should exist");
    let content = std::fs::read_to_string(&workflow_path).unwrap();
    assert!(!content.is_empty(), "workflow.md should not be empty");
}

#[test]
fn test_init_creates_gitignore() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions::default();
    create_workflow_structure(cwd, &options).unwrap();

    let gitignore_path = cwd.join(".harness-cli/.gitignore");
    assert!(gitignore_path.is_file(), ".gitignore should exist");
}

#[test]
fn test_init_creates_config_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions::default();
    create_workflow_structure(cwd, &options).unwrap();

    let config_path = cwd.join(".harness-cli/config.yaml");
    assert!(config_path.is_file(), "config.yaml should exist");
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(!content.is_empty(), "config.yaml should not be empty");
}

#[test]
fn test_init_creates_workspace_index() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions::default();
    create_workflow_structure(cwd, &options).unwrap();

    let index_path = cwd.join(".harness-cli/workspace/index.md");
    assert!(index_path.is_file(), "workspace/index.md should exist");
}

#[test]
fn test_init_multi_agent_creates_worktree_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions {
        multi_agent: true,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    let worktree_path = cwd.join(".harness-cli/worktree.yaml");
    assert!(
        worktree_path.is_file(),
        "worktree.yaml should exist when multi_agent is true"
    );
}

#[test]
fn test_init_no_multi_agent_skips_worktree_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions {
        multi_agent: false,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    let worktree_path = cwd.join(".harness-cli/worktree.yaml");
    assert!(
        !worktree_path.exists(),
        "worktree.yaml should NOT exist when multi_agent is false"
    );
}

#[test]
fn test_init_scripts_are_present() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions::default();
    create_workflow_structure(cwd, &options).unwrap();

    let scripts_dir = cwd.join(".harness-cli/scripts");
    assert!(scripts_dir.is_dir(), "scripts/ dir should exist");

    // Check that at least some known scripts exist
    let expected_scripts = ["task.py", "get_context.py", "get_developer.py"];
    for script in &expected_scripts {
        assert!(
            scripts_dir.join(script).is_file(),
            "Expected script '{}' to exist",
            script
        );
    }
}

#[test]
fn test_init_guides_always_created() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    // Even for Backend-only, guides should be created
    let options = WorkflowOptions {
        project_type: ProjectType::Backend,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    let guides_dir = cwd.join(".harness-cli/spec/guides");
    assert!(guides_dir.is_dir(), "spec/guides/ should always exist");
    assert!(
        guides_dir.join("index.md").is_file(),
        "spec/guides/index.md should exist"
    );
}

#[test]
fn test_configure_platform_creates_correct_dirs() {
    // Test a few representative platforms
    let platforms_and_dirs: Vec<(AITool, &str)> = vec![
        (AITool::ClaudeCode, ".claude"),
        (AITool::IFlow, ".iflow"),
        (AITool::Codex, ".codex"),
    ];

    for (platform, expected_dir) in &platforms_and_dirs {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();

        // Set force mode so we don't get interactive prompts
        set_write_mode(WriteMode::Force);
        configurators::configure_platform(*platform, cwd).unwrap();
        // Reset to default
        set_write_mode(WriteMode::Ask);

        assert!(
            cwd.join(expected_dir).is_dir(),
            "Expected config dir '{}' to exist after configuring {:?}",
            expected_dir,
            platform
        );
    }
}

#[test]
fn test_force_mode_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.txt");

    // Write original content
    std::fs::write(&file, "original content").unwrap();

    // Set force mode and write different content
    set_write_mode(WriteMode::Force);
    let result = write_file(&file, "new content", false).unwrap();
    set_write_mode(WriteMode::Ask);

    assert!(result, "Force mode should report file was written");
    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        content, "new content",
        "File should be overwritten in force mode"
    );
}

#[test]
fn test_skip_mode_preserves() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.txt");

    // Write original content
    std::fs::write(&file, "original content").unwrap();

    // Set skip mode and try to write different content
    set_write_mode(WriteMode::Skip);
    let result = write_file(&file, "new content", false).unwrap();
    set_write_mode(WriteMode::Ask);

    assert!(!result, "Skip mode should report file was NOT written");
    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        content, "original content",
        "Original content should be preserved in skip mode"
    );
}

#[test]
fn test_version_constant_is_valid_semver() {
    assert!(!VERSION.is_empty(), "VERSION constant should not be empty");
    // Basic semver: should have at least two dots for x.y.z
    let base = VERSION.split('-').next().unwrap();
    let parts: Vec<&str> = base.split('.').collect();
    assert!(
        parts.len() >= 3,
        "VERSION '{}' should have at least 3 numeric parts (x.y.z)",
        VERSION
    );
    for part in &parts {
        assert!(
            part.parse::<u64>().is_ok(),
            "VERSION part '{}' should be numeric",
            part
        );
    }
}

// =========================================================================
// 12. Template Hash Tests
// =========================================================================

#[test]
fn test_compute_hash_deterministic() {
    let h1 = compute_hash("hello world");
    let h2 = compute_hash("hello world");
    assert_eq!(h1, h2, "Same content should produce same hash");
}

#[test]
fn test_compute_hash_differs_for_different_content() {
    let h1 = compute_hash("hello");
    let h2 = compute_hash("world");
    assert_ne!(h1, h2, "Different content should produce different hashes");
}

#[test]
fn test_hash_tracking_after_init() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    // Create workflow structure
    let options = WorkflowOptions::default();
    create_workflow_structure(cwd, &options).unwrap();

    // Initialize hashes
    let managed = vec![".harness-cli"];
    let count = initialize_hashes(cwd, &managed);
    assert!(count > 0, "Should hash at least one file");

    // Verify hash file exists
    let hashes_file = cwd.join(".harness-cli/.template-hashes.json");
    assert!(hashes_file.is_file(), "Hash file should exist after init");

    // Load and verify
    let hashes = load_hashes(cwd);
    assert!(!hashes.is_empty(), "Loaded hashes should not be empty");
}

#[test]
fn test_template_hash_detects_modification() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    // Create .harness-cli directory
    ensure_dir(&cwd.join(".harness-cli")).unwrap();

    // Write a file and record its hash
    let rel_path = ".harness-cli/test-file.txt";
    let original = "original template content";
    std::fs::write(cwd.join(rel_path), original).unwrap();

    let mut files = HashMap::new();
    files.insert(rel_path.to_string(), original.to_string());
    update_hashes(cwd, &files);

    // Modify the file
    std::fs::write(cwd.join(rel_path), "user modified content").unwrap();

    // Check: should detect modification
    let hashes = load_hashes(cwd);
    assert!(
        is_template_modified(cwd, rel_path, &hashes),
        "Modified file should be detected as modified"
    );
}

#[test]
fn test_template_hash_unmodified() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    // Create .harness-cli directory
    ensure_dir(&cwd.join(".harness-cli")).unwrap();

    // Write a file and record its hash
    let rel_path = ".harness-cli/test-file.txt";
    let original = "template content stays the same";
    std::fs::write(cwd.join(rel_path), original).unwrap();

    let mut files = HashMap::new();
    files.insert(rel_path.to_string(), original.to_string());
    update_hashes(cwd, &files);

    // Don't modify the file -- check: should NOT detect modification
    let hashes = load_hashes(cwd);
    assert!(
        !is_template_modified(cwd, rel_path, &hashes),
        "Unmodified file should not be detected as modified"
    );
}

#[test]
fn test_user_deleted_file_not_readded() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    // Create .harness-cli directory
    ensure_dir(&cwd.join(".harness-cli")).unwrap();

    // Write a file and record its hash
    let rel_path = ".harness-cli/deletable.txt";
    let content = "some content";
    std::fs::write(cwd.join(rel_path), content).unwrap();

    let mut files = HashMap::new();
    files.insert(rel_path.to_string(), content.to_string());
    update_hashes(cwd, &files);

    // Delete the file
    std::fs::remove_file(cwd.join(rel_path)).unwrap();

    // is_template_modified returns false when file doesn't exist
    // (it can't be "modified" if it doesn't exist)
    let hashes = load_hashes(cwd);
    assert!(
        !is_template_modified(cwd, rel_path, &hashes),
        "Deleted file should not be considered modified"
    );

    // The hash still exists in the store (file was tracked)
    assert!(
        hashes.contains_key(rel_path),
        "Hash entry should persist even after file deletion"
    );
}

#[test]
fn test_save_and_load_hashes_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    ensure_dir(&cwd.join(".harness-cli")).unwrap();

    let mut hashes = HashMap::new();
    hashes.insert("file1.txt".to_string(), "hash1".to_string());
    hashes.insert("file2.txt".to_string(), "hash2".to_string());

    save_hashes(cwd, &hashes);
    let loaded = load_hashes(cwd);

    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.get("file1.txt").unwrap(), "hash1");
    assert_eq!(loaded.get("file2.txt").unwrap(), "hash2");
}

// =========================================================================
// 13. Additional version comparison tests
// =========================================================================

#[test]
fn test_compare_versions_equal() {
    assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
}

#[test]
fn test_compare_versions_major() {
    assert_eq!(compare_versions("2.0.0", "1.0.0"), Ordering::Greater);
}

#[test]
fn test_compare_versions_minor() {
    assert_eq!(compare_versions("1.0.0", "1.1.0"), Ordering::Less);
}

#[test]
fn test_compare_versions_patch() {
    assert_eq!(compare_versions("1.0.1", "1.0.0"), Ordering::Greater);
}

#[test]
fn test_compare_versions_prerelease_alpha_before_beta() {
    assert_eq!(
        compare_versions("1.0.0-alpha", "1.0.0-beta"),
        Ordering::Less
    );
}

#[test]
fn test_compare_versions_beta_before_rc() {
    assert_eq!(
        compare_versions("0.3.0-beta.16", "0.3.0-rc.0"),
        Ordering::Less
    );
}

#[test]
fn test_compare_versions_rc_before_release() {
    assert_eq!(compare_versions("0.3.0-rc.0", "0.3.0"), Ordering::Less);
}

// =========================================================================
// 14. Path helper tests
// =========================================================================

#[test]
fn test_get_workspace_dir() {
    assert_eq!(get_workspace_dir("alice"), ".harness-cli/workspace/alice");
}

#[test]
fn test_get_task_dir() {
    assert_eq!(
        get_task_dir("04-10-my-feature"),
        ".harness-cli/tasks/04-10-my-feature"
    );
}

#[test]
fn test_get_archive_dir() {
    assert_eq!(get_archive_dir(), ".harness-cli/tasks/archive");
}

#[test]
fn test_constructed_paths_consistent_with_dir_names() {
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

// =========================================================================
// 15. Platform helpers
// =========================================================================

#[test]
fn test_platform_ids_returns_all() {
    let ids = configurators::platform_ids();
    assert_eq!(
        ids.len(),
        AITool::all().len(),
        "platform_ids() should return all AI tools"
    );
}

#[test]
fn test_config_dirs_all_start_with_dot() {
    let dirs = configurators::config_dirs();
    for dir in &dirs {
        assert!(
            dir.starts_with('.'),
            "Config dir '{}' should start with '.'",
            dir
        );
    }
}

#[test]
fn test_all_managed_dirs_includes_harness_cli() {
    let dirs = configurators::all_managed_dirs();
    assert!(
        dirs.contains(&".harness-cli".to_string()),
        "all_managed_dirs should include '.harness-cli'"
    );
}

#[test]
fn test_get_configured_platforms_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let platforms = configurators::get_configured_platforms(tmp.path());
    assert!(
        platforms.is_empty(),
        "Empty directory should have no configured platforms"
    );
}

#[test]
fn test_get_configured_platforms_detects_claude() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".claude")).unwrap();
    let platforms = configurators::get_configured_platforms(tmp.path());
    assert!(
        platforms.contains(&AITool::ClaudeCode),
        "Should detect Claude Code when .claude/ exists"
    );
}

#[test]
fn test_get_platforms_with_python_hooks() {
    let hooks_platforms = configurators::get_platforms_with_python_hooks();
    // Claude, iFlow, Codex, Copilot should have python hooks
    let has_claude = hooks_platforms.contains(&AITool::ClaudeCode);
    let has_iflow = hooks_platforms.contains(&AITool::IFlow);
    let has_codex = hooks_platforms.contains(&AITool::Codex);
    let has_copilot = hooks_platforms.contains(&AITool::Copilot);

    assert!(has_claude, "Claude should have python hooks");
    assert!(has_iflow, "iFlow should have python hooks");
    assert!(has_codex, "Codex should have python hooks");
    assert!(has_copilot, "Copilot should have python hooks");
}

#[test]
fn test_resolve_cli_flag() {
    assert_eq!(
        configurators::resolve_cli_flag("claude"),
        Some(AITool::ClaudeCode)
    );
    assert_eq!(
        configurators::resolve_cli_flag("cursor"),
        Some(AITool::Cursor)
    );
    assert_eq!(configurators::resolve_cli_flag("nonexistent"), None);
}

#[test]
fn test_codex_managed_paths_includes_agents() {
    let paths = get_managed_paths(AITool::Codex);
    assert!(
        paths.contains(&".codex"),
        "Codex managed paths should include .codex"
    );
    assert!(
        paths.contains(&".agents/skills"),
        "Codex managed paths should include .agents/skills"
    );
}

#[test]
fn test_copilot_managed_paths() {
    let paths = get_managed_paths(AITool::Copilot);
    assert!(paths.contains(&".github/copilot"));
    assert!(paths.contains(&".github/hooks"));
    assert!(paths.contains(&".github/prompts"));
}

// =========================================================================
// 16. is_managed_root_dir tests
// =========================================================================

#[test]
fn test_is_managed_root_dir_harness_cli() {
    assert!(
        configurators::is_managed_root_dir(".harness-cli"),
        ".harness-cli should be a managed root dir"
    );
}

#[test]
fn test_is_managed_root_dir_claude() {
    assert!(
        configurators::is_managed_root_dir(".claude"),
        ".claude should be a managed root dir"
    );
}

#[test]
fn test_is_managed_root_dir_rejects_unknown() {
    assert!(
        !configurators::is_managed_root_dir(".random-dir"),
        ".random-dir should NOT be a managed root dir"
    );
}

// =========================================================================
// 17. File writer edge cases
// =========================================================================

#[test]
fn test_write_new_file_creates_it() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("brand-new.txt");
    assert!(!file.exists());

    // write_file doesn't need special mode for new files
    set_write_mode(WriteMode::Ask);
    let result = write_file(&file, "hello", false).unwrap();
    set_write_mode(WriteMode::Ask);

    assert!(result, "Should report success for new file");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello");
}

#[test]
fn test_write_identical_content_returns_false() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("same.txt");
    std::fs::write(&file, "identical").unwrap();

    set_write_mode(WriteMode::Force);
    let result = write_file(&file, "identical", false).unwrap();
    set_write_mode(WriteMode::Ask);

    assert!(!result, "Identical content should return false (no change)");
}

#[test]
fn test_ensure_dir_nested() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("a").join("b").join("c");
    assert!(!nested.exists());

    ensure_dir(&nested).unwrap();
    assert!(nested.is_dir());
}

// =========================================================================
// 18. Init tool choices
// =========================================================================

#[test]
fn test_init_tool_choices_has_entries() {
    let choices = configurators::get_init_tool_choices();
    assert_eq!(
        choices.len(),
        AITool::all().len(),
        "Should have one choice per AI tool"
    );
}

#[test]
fn test_init_tool_choices_claude_default_checked() {
    let choices = configurators::get_init_tool_choices();
    let claude_choice = choices
        .iter()
        .find(|c| c.platform_id == AITool::ClaudeCode)
        .expect("Should have Claude choice");
    assert!(
        claude_choice.default_checked,
        "Claude should be default checked"
    );
}

#[test]
fn test_init_tool_choices_cursor_default_checked() {
    let choices = configurators::get_init_tool_choices();
    let cursor_choice = choices
        .iter()
        .find(|c| c.platform_id == AITool::Cursor)
        .expect("Should have Cursor choice");
    assert!(
        cursor_choice.default_checked,
        "Cursor should be default checked"
    );
}

// =========================================================================
// 19. Copilot-specific template checks
// =========================================================================

#[test]
fn test_copilot_has_prompts() {
    let prompts = copilot::get_all_prompts();
    assert!(
        !prompts.is_empty(),
        "Copilot should have at least one prompt template"
    );
    for prompt in &prompts {
        assert!(
            !prompt.name.is_empty(),
            "Copilot prompt should have a non-empty name"
        );
        assert!(
            !prompt.content.is_empty(),
            "Copilot prompt '{}' should have non-empty content",
            prompt.name
        );
    }
}

#[test]
fn test_copilot_has_hooks() {
    let hooks = copilot::get_all_hooks();
    assert!(!hooks.is_empty(), "Copilot should have at least one hook");
}

#[test]
fn test_copilot_hooks_config_valid_json() {
    let config = copilot::get_hooks_config();
    assert!(!config.is_empty(), "Copilot hooks.json should not be empty");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&config);
    assert!(
        parsed.is_ok(),
        "Copilot hooks.json should be valid JSON: {:?}",
        parsed.err()
    );
}

// =========================================================================
// 20. Codex-specific template checks
// =========================================================================

#[test]
fn test_codex_has_agents() {
    let agents = codex::get_all_agents();
    assert!(
        !agents.is_empty(),
        "Codex should have at least one agent template"
    );
}

#[test]
fn test_codex_has_hooks() {
    let hooks = codex::get_all_hooks();
    assert!(!hooks.is_empty(), "Codex should have at least one hook");
}

#[test]
fn test_codex_hooks_config_valid_json() {
    let config = codex::get_hooks_config();
    assert!(!config.is_empty(), "Codex hooks.json should not be empty");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&config);
    assert!(
        parsed.is_ok(),
        "Codex hooks.json should be valid JSON: {:?}",
        parsed.err()
    );
}

#[test]
fn test_codex_config_toml_not_empty() {
    let config = codex::get_config_template();
    assert!(
        !config.content.is_empty(),
        "Codex config.toml should not be empty"
    );
}

#[test]
fn test_codex_has_codex_specific_skills() {
    let skills = codex::get_all_codex_skills();
    assert!(
        !skills.is_empty(),
        "Codex should have at least one codex-specific skill"
    );
}

// =========================================================================
// 21. Claude and iFlow agent/hook template checks
// =========================================================================

#[test]
fn test_claude_has_agents() {
    let agents = claude::get_all_agents();
    assert!(
        !agents.is_empty(),
        "Claude should have at least one agent template"
    );
    for agent in &agents {
        assert!(!agent.name.is_empty(), "Agent name should not be empty");
        assert!(
            !agent.content.is_empty(),
            "Agent '{}' content should not be empty",
            agent.name
        );
    }
}

#[test]
fn test_claude_has_hooks() {
    let hooks = claude::get_all_hooks();
    assert!(
        !hooks.is_empty(),
        "Claude should have at least one hook template"
    );
}

#[test]
fn test_iflow_has_commands() {
    let commands = iflow::get_all_commands();
    assert!(
        !commands.is_empty(),
        "iFlow should have at least one command template"
    );
}

#[test]
fn test_iflow_has_agents() {
    let agents = iflow::get_all_agents();
    assert!(
        !agents.is_empty(),
        "iFlow should have at least one agent template"
    );
}

#[test]
fn test_iflow_has_hooks() {
    let hooks = iflow::get_all_hooks();
    assert!(
        !hooks.is_empty(),
        "iFlow should have at least one hook template"
    );
}

// =========================================================================
// 22. Harness CLI template content checks
// =========================================================================

#[test]
fn test_workflow_md_template_not_empty() {
    let content = hc_templates::workflow_md_template();
    assert!(
        !content.is_empty(),
        "workflow.md template should not be empty"
    );
}

#[test]
fn test_config_yaml_template_not_empty() {
    let content = hc_templates::config_yaml_template();
    assert!(
        !content.is_empty(),
        "config.yaml template should not be empty"
    );
}

#[test]
fn test_gitignore_template_not_empty() {
    let content = hc_templates::gitignore_template();
    assert!(
        !content.is_empty(),
        ".gitignore template should not be empty"
    );
}

// =========================================================================
// 23. Project detector type descriptions
// =========================================================================

#[test]
fn test_project_type_descriptions() {
    use harness_cli::utils::project_detector::get_project_type_description;

    assert!(
        get_project_type_description(ProjectType::Frontend).contains("Frontend"),
        "Frontend description should mention Frontend"
    );
    assert!(
        get_project_type_description(ProjectType::Backend).contains("Backend"),
        "Backend description should mention Backend"
    );
    assert!(
        get_project_type_description(ProjectType::Fullstack).contains("Fullstack"),
        "Fullstack description should mention Fullstack"
    );
    assert!(
        get_project_type_description(ProjectType::Unknown).contains("Unknown"),
        "Unknown description should mention Unknown"
    );
}

// =========================================================================
// 24. Project detection in empty directory
// =========================================================================

#[test]
fn test_detect_project_type_empty_dir() {
    use harness_cli::utils::project_detector::detect_project_type;

    let tmp = tempfile::tempdir().unwrap();
    let result = detect_project_type(tmp.path());
    assert_eq!(
        result,
        ProjectType::Unknown,
        "Empty directory should be Unknown project type"
    );
}

#[test]
fn test_detect_project_type_backend() {
    use harness_cli::utils::project_detector::detect_project_type;

    let tmp = tempfile::tempdir().unwrap();
    // Create a Cargo.toml to indicate backend
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )
    .unwrap();

    let result = detect_project_type(tmp.path());
    assert_eq!(
        result,
        ProjectType::Backend,
        "Directory with Cargo.toml should be Backend"
    );
}

#[test]
fn test_detect_project_type_frontend() {
    use harness_cli::utils::project_detector::detect_project_type;

    let tmp = tempfile::tempdir().unwrap();
    // Create a vite.config.ts to indicate frontend
    std::fs::write(tmp.path().join("vite.config.ts"), "export default {}").unwrap();

    let result = detect_project_type(tmp.path());
    // vite.config.ts is a frontend indicator
    assert!(
        result == ProjectType::Frontend || result == ProjectType::Fullstack,
        "Directory with vite.config.ts should be Frontend or Fullstack, got {:?}",
        result
    );
}

// =========================================================================
// 25. Sanitize package name
// =========================================================================

#[test]
fn test_sanitize_pkg_name_strips_scope() {
    use harness_cli::utils::project_detector::sanitize_pkg_name;

    assert_eq!(sanitize_pkg_name("@scope/package"), "package");
    assert_eq!(sanitize_pkg_name("simple-name"), "simple-name");
    assert_eq!(sanitize_pkg_name("@org/my-lib"), "my-lib");
}

// =========================================================================
// 26. Migration function edge cases
// =========================================================================

#[test]
fn test_get_all_migration_versions_non_empty() {
    let versions = migrations::get_all_migration_versions();
    assert!(
        !versions.is_empty(),
        "Should have at least one migration version"
    );
}

#[test]
fn test_get_migration_summary_zero_range() {
    let summary = migrations::get_migration_summary("0.3.0", "0.3.0");
    assert_eq!(summary.renames, 0);
    assert_eq!(summary.deletes, 0);
    assert_eq!(summary.safe_file_deletes, 0);
}

#[test]
fn test_get_migration_metadata_empty_range() {
    let metadata = migrations::get_migration_metadata("0.3.0", "0.3.0");
    assert!(
        metadata.changelog.is_empty(),
        "Empty range should have no changelog"
    );
}

#[test]
fn test_get_migration_metadata_full_range() {
    let metadata = migrations::get_migration_metadata("0.0.0", "99.99.99");
    // Should not panic and should aggregate data from all manifests
    let _ = metadata.breaking;
    let _ = metadata.recommend_migrate;
    // We expect at least some changelog entries given the many manifest files
    assert!(
        !metadata.changelog.is_empty(),
        "Full range should produce some changelog entries"
    );
}

// =========================================================================
// 27. OpenCode returns None for collect_templates
// =========================================================================

#[test]
fn test_opencode_no_template_collection() {
    let result = configurators::collect_platform_templates(AITool::OpenCode);
    assert!(
        result.is_none(),
        "OpenCode should return None for collect_platform_templates (uses plugin system)"
    );
}

// =========================================================================
// 28. AITool Display trait
// =========================================================================

#[test]
fn test_ai_tool_display() {
    assert_eq!(AITool::ClaudeCode.to_string(), "claude-code");
    assert_eq!(AITool::CodeBuddy.to_string(), "codebuddy");
    assert_eq!(AITool::Copilot.to_string(), "copilot");
    assert_eq!(AITool::IFlow.to_string(), "iflow");
}

// =========================================================================
// 29. All platforms have template collection except OpenCode
// =========================================================================

#[test]
fn test_all_platforms_except_opencode_have_templates() {
    for tool in AITool::all() {
        let result = configurators::collect_platform_templates(*tool);
        if *tool == AITool::OpenCode {
            assert!(result.is_none(), "OpenCode should return None");
        } else {
            assert!(
                result.is_some(),
                "{:?} should support template collection",
                tool
            );
            let templates = result.unwrap();
            assert!(
                !templates.is_empty(),
                "{:?} should have at least one template",
                tool
            );
        }
    }
}

// =========================================================================
// 30. Skip spec templates option
// =========================================================================

#[test]
fn test_skip_spec_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();

    let options = WorkflowOptions {
        skip_spec_templates: true,
        ..Default::default()
    };
    create_workflow_structure(cwd, &options).unwrap();

    // Core directories should still exist
    assert!(cwd.join(".harness-cli").is_dir());
    assert!(cwd.join(".harness-cli/scripts").is_dir());
    assert!(cwd.join(".harness-cli/workspace").is_dir());
    assert!(cwd.join(".harness-cli/tasks").is_dir());

    // Spec directory should NOT be created when skipping
    assert!(
        !cwd.join(".harness-cli/spec").exists(),
        "spec/ should NOT exist when skip_spec_templates is true"
    );
}
