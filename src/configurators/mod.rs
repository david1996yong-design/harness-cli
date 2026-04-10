//! Platform configurators and registry.
//!
//! Each platform has a `configure(cwd)` function that copies embedded templates
//! to the user's project directory, plus an optional `collect_templates()`
//! function for update tracking.

pub mod shared;
pub mod workflow;

pub mod antigravity;
pub mod claude;
pub mod codebuddy;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod iflow;
pub mod kilo;
pub mod kiro;
pub mod opencode;
pub mod qoder;
pub mod windsurf;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;

use crate::types::ai_tools::{get_managed_paths, get_tool_config, AITool, CliFlag};

// ---------------------------------------------------------------------------
// Public helpers (derived from AI_TOOLS registry)
// ---------------------------------------------------------------------------

/// All platform IDs.
pub fn platform_ids() -> &'static [AITool] {
    AITool::all()
}

/// All platform config directory names.
pub fn config_dirs() -> Vec<&'static str> {
    AITool::all()
        .iter()
        .map(|id| get_tool_config(*id).config_dir)
        .collect()
}

/// All directories managed by Harness CLI (including `.harness-cli` itself).
pub fn all_managed_dirs() -> Vec<String> {
    let mut dirs: Vec<String> = vec![".harness-cli".to_string()];
    let mut seen = HashSet::new();
    seen.insert(".harness-cli".to_string());

    for id in AITool::all() {
        for p in get_managed_paths(*id) {
            let s = p.to_string();
            if seen.insert(s.clone()) {
                dirs.push(s);
            }
        }
    }
    dirs
}

/// Detect which platforms are configured by checking for `config_dir` existence.
pub fn get_configured_platforms(cwd: &Path) -> HashSet<AITool> {
    let mut platforms = HashSet::new();
    for id in AITool::all() {
        let config_dir = get_tool_config(*id).config_dir;
        if cwd.join(config_dir).exists() {
            platforms.insert(*id);
        }
    }
    platforms
}

/// Get platform IDs that have Python hooks.
pub fn get_platforms_with_python_hooks() -> Vec<AITool> {
    AITool::all()
        .iter()
        .copied()
        .filter(|id| get_tool_config(*id).has_python_hooks)
        .collect()
}

/// Check if a path starts with any managed directory.
pub fn is_managed_path(dir_path: &str) -> bool {
    let normalized = dir_path.replace('\\', "/");
    all_managed_dirs().iter().any(|d| {
        normalized.starts_with(&format!("{}/", d)) || normalized == *d
    })
}

/// Check if a directory name is a managed root directory.
pub fn is_managed_root_dir(dir_name: &str) -> bool {
    all_managed_dirs().iter().any(|d| d == dir_name)
}

/// Configure a platform by copying its templates to the project.
pub fn configure_platform(platform: AITool, cwd: &Path) -> Result<()> {
    match platform {
        AITool::ClaudeCode => claude::configure(cwd),
        AITool::Cursor => cursor::configure(cwd),
        AITool::OpenCode => opencode::configure(cwd),
        AITool::IFlow => iflow::configure(cwd),
        AITool::Codex => codex::configure(cwd),
        AITool::Kilo => kilo::configure(cwd),
        AITool::Kiro => kiro::configure(cwd),
        AITool::Gemini => gemini::configure(cwd),
        AITool::Antigravity => antigravity::configure(cwd),
        AITool::Windsurf => windsurf::configure(cwd),
        AITool::Qoder => qoder::configure(cwd),
        AITool::CodeBuddy => codebuddy::configure(cwd),
        AITool::Copilot => copilot::configure(cwd),
    }
}

/// Collect template files for a specific platform (for update tracking).
///
/// Returns `None` if the platform doesn't support template tracking.
pub fn collect_platform_templates(platform: AITool) -> Option<HashMap<String, String>> {
    match platform {
        AITool::ClaudeCode => Some(claude::collect_templates()),
        AITool::Cursor => Some(cursor::collect_templates()),
        AITool::OpenCode => None, // OpenCode uses plugin system, handled separately
        AITool::IFlow => Some(iflow::collect_templates()),
        AITool::Codex => Some(codex::collect_templates()),
        AITool::Kilo => Some(kilo::collect_templates()),
        AITool::Kiro => Some(kiro::collect_templates()),
        AITool::Gemini => Some(gemini::collect_templates()),
        AITool::Antigravity => Some(antigravity::collect_templates()),
        AITool::Windsurf => Some(windsurf::collect_templates()),
        AITool::Qoder => Some(qoder::collect_templates()),
        AITool::CodeBuddy => Some(codebuddy::collect_templates()),
        AITool::Copilot => Some(copilot::collect_templates()),
    }
}

/// A choice entry for the interactive init prompt.
#[derive(Debug, Clone)]
pub struct InitToolChoice {
    pub key: CliFlag,
    pub name: &'static str,
    pub default_checked: bool,
    pub platform_id: AITool,
}

/// Build choices for the interactive init prompt, derived from AI_TOOLS registry.
pub fn get_init_tool_choices() -> Vec<InitToolChoice> {
    AITool::all()
        .iter()
        .map(|id| {
            let cfg = get_tool_config(*id);
            InitToolChoice {
                key: cfg.cli_flag,
                name: cfg.name,
                default_checked: cfg.default_checked,
                platform_id: *id,
            }
        })
        .collect()
}

/// Resolve CLI flag name to AITool id (e.g., `"claude"` -> `AITool::ClaudeCode`).
pub fn resolve_cli_flag(flag: &str) -> Option<AITool> {
    AITool::all()
        .iter()
        .copied()
        .find(|id| get_tool_config(*id).cli_flag.as_str() == flag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Registry / index tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_platform_ids_count() {
        assert_eq!(
            platform_ids().len(),
            AITool::all().len(),
            "platform_ids() length should match AITool::all()"
        );
    }

    #[test]
    fn test_config_dirs_length() {
        assert_eq!(
            config_dirs().len(),
            platform_ids().len(),
            "config_dirs() length should match platform_ids()"
        );
    }

    #[test]
    fn test_all_managed_dirs_starts_with_harness() {
        let dirs = all_managed_dirs();
        assert_eq!(
            dirs[0], ".harness-cli",
            "First managed dir should be '.harness-cli'"
        );
    }

    #[test]
    fn test_all_managed_dirs_no_duplicates() {
        let dirs = all_managed_dirs();
        let unique: std::collections::HashSet<&String> = dirs.iter().collect();
        assert_eq!(dirs.len(), unique.len(), "Managed dirs should have no duplicates");
    }

    #[test]
    fn test_is_managed_path_matches() {
        assert!(
            is_managed_path(".claude/commands/foo"),
            "'.claude/commands/foo' should be a managed path"
        );
    }

    #[test]
    fn test_is_managed_path_exact() {
        assert!(
            is_managed_path(".harness-cli"),
            "'.harness-cli' should be a managed path"
        );
    }

    #[test]
    fn test_is_managed_path_rejects_unrelated() {
        assert!(
            !is_managed_path("random/path"),
            "'random/path' should not be a managed path"
        );
    }

    #[test]
    fn test_is_managed_path_rejects_prefix_similar() {
        assert!(
            !is_managed_path(".claude-code"),
            "'.claude-code' should not be a managed path (not exact match)"
        );
    }

    #[test]
    fn test_is_managed_path_empty() {
        assert!(!is_managed_path(""), "Empty string should not be a managed path");
    }

    #[test]
    fn test_is_managed_path_windows_backslash() {
        assert!(
            is_managed_path(".claude\\commands"),
            "'.claude\\\\commands' should be normalized and match as managed path"
        );
    }

    #[test]
    fn test_resolve_cli_flag_claude() {
        assert_eq!(
            resolve_cli_flag("claude"),
            Some(AITool::ClaudeCode),
            "'claude' should resolve to ClaudeCode"
        );
    }

    #[test]
    fn test_resolve_cli_flag_cursor() {
        assert_eq!(
            resolve_cli_flag("cursor"),
            Some(AITool::Cursor),
            "'cursor' should resolve to Cursor"
        );
    }

    #[test]
    fn test_resolve_cli_flag_unknown() {
        assert_eq!(
            resolve_cli_flag("unknown"),
            None,
            "'unknown' should resolve to None"
        );
    }

    #[test]
    fn test_resolve_cli_flag_empty() {
        assert_eq!(resolve_cli_flag(""), None, "Empty string should resolve to None");
    }

    #[test]
    fn test_resolve_cli_flag_case_sensitive() {
        assert_eq!(
            resolve_cli_flag("Claude"),
            None,
            "'Claude' (capitalized) should resolve to None (case sensitive)"
        );
    }

    #[test]
    fn test_get_init_tool_choices_count() {
        let choices = get_init_tool_choices();
        assert_eq!(
            choices.len(),
            AITool::all().len(),
            "Should have one choice per platform"
        );
    }

    #[test]
    fn test_get_init_tool_choices_has_fields() {
        for choice in get_init_tool_choices() {
            assert!(
                !choice.name.is_empty(),
                "Choice for {:?} should have non-empty name",
                choice.platform_id
            );
            // key and platform_id are typed, so they always have values
            let _ = choice.key;
            let _ = choice.default_checked;
            let _ = choice.platform_id;
        }
    }

    #[test]
    fn test_tool_choices_roundtrip() {
        for choice in get_init_tool_choices() {
            let resolved = resolve_cli_flag(choice.key.as_str());
            assert_eq!(
                resolved,
                Some(choice.platform_id),
                "CLI flag '{}' should resolve back to {:?}",
                choice.key.as_str(),
                choice.platform_id
            );
        }
    }

    // -----------------------------------------------------------------------
    // Configure platform tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_configure_claude_creates_dir() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::ClaudeCode, cwd).unwrap();
        assert!(cwd.join(".claude").is_dir());
    }

    #[test]
    fn test_configure_cursor_creates_dir() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::Cursor, cwd).unwrap();
        assert!(cwd.join(".cursor").is_dir());
    }

    #[test]
    fn test_configure_codex_creates_skills() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::Codex, cwd).unwrap();
        assert!(cwd.join(".agents/skills").is_dir());
        assert!(cwd.join(".codex").is_dir());
    }

    #[test]
    fn test_configure_kiro_creates_skills() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::Kiro, cwd).unwrap();
        assert!(cwd.join(".kiro/skills").is_dir());
    }

    #[test]
    fn test_configure_antigravity_creates_workflows() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::Antigravity, cwd).unwrap();
        assert!(cwd.join(".agent/workflows").is_dir());
    }

    #[test]
    fn test_configure_windsurf_creates_workflows() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::Windsurf, cwd).unwrap();
        assert!(cwd.join(".windsurf/workflows").is_dir());
    }

    #[test]
    fn test_configure_copilot_creates_hooks() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        configure_platform(AITool::Copilot, cwd).unwrap();
        assert!(cwd.join(".github/copilot/hooks").is_dir());
    }

    #[test]
    fn test_get_configured_platforms_empty() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let platforms = get_configured_platforms(cwd);
        assert!(
            platforms.is_empty(),
            "Empty dir should have no configured platforms"
        );
    }

    #[test]
    fn test_get_configured_platforms_claude() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        std::fs::create_dir_all(cwd.join(".claude")).unwrap();
        let platforms = get_configured_platforms(cwd);
        assert!(
            platforms.contains(&AITool::ClaudeCode),
            "Should detect ClaudeCode when .claude dir exists"
        );
    }

    #[test]
    fn test_get_configured_platforms_multiple() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        std::fs::create_dir_all(cwd.join(".claude")).unwrap();
        std::fs::create_dir_all(cwd.join(".cursor")).unwrap();
        let platforms = get_configured_platforms(cwd);
        assert!(platforms.contains(&AITool::ClaudeCode));
        assert!(platforms.contains(&AITool::Cursor));
    }

    // -----------------------------------------------------------------------
    // collect_platform_templates tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_collect_claude_non_empty() {
        let result = collect_platform_templates(AITool::ClaudeCode);
        assert!(result.is_some(), "Claude should return Some");
        let map = result.unwrap();
        assert!(!map.is_empty(), "Claude templates should be non-empty");
    }

    #[test]
    fn test_collect_opencode_none() {
        let result = collect_platform_templates(AITool::OpenCode);
        assert!(
            result.is_none(),
            "OpenCode should return None (no template tracking)"
        );
    }

    #[test]
    fn test_collect_codex_tracks_skills() {
        let result = collect_platform_templates(AITool::Codex);
        assert!(result.is_some(), "Codex should return Some");
        let map = result.unwrap();
        let has_skills = map.keys().any(|k| k.starts_with(".agents/skills/"));
        assert!(has_skills, "Codex templates should track .agents/skills/ paths");
    }

    #[test]
    fn test_collect_copilot_tracks_hooks() {
        let result = collect_platform_templates(AITool::Copilot);
        assert!(result.is_some(), "Copilot should return Some");
        let map = result.unwrap();
        let has_github = map.keys().any(|k| k.starts_with(".github/"));
        assert!(has_github, "Copilot templates should track .github/ paths");
    }
}
