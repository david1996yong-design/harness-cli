//! AI Tool Types and Registry
//!
//! Defines supported AI coding tools and which command templates they can use.

use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// AITool enum
// ---------------------------------------------------------------------------

/// Supported AI coding tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AITool {
    ClaudeCode,
    Cursor,
    OpenCode,
    IFlow,
    Codex,
    Kilo,
    Kiro,
    Gemini,
    Antigravity,
    Windsurf,
    Qoder,
    CodeBuddy,
    Copilot,
}

impl AITool {
    /// Returns a slice containing every [`AITool`] variant.
    pub fn all() -> &'static [AITool] {
        &[
            AITool::ClaudeCode,
            AITool::Cursor,
            AITool::OpenCode,
            AITool::IFlow,
            AITool::Codex,
            AITool::Kilo,
            AITool::Kiro,
            AITool::Gemini,
            AITool::Antigravity,
            AITool::Windsurf,
            AITool::Qoder,
            AITool::CodeBuddy,
            AITool::Copilot,
        ]
    }

    /// The kebab-case identifier used in configuration files and serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            AITool::ClaudeCode => "claude-code",
            AITool::Cursor => "cursor",
            AITool::OpenCode => "opencode",
            AITool::IFlow => "iflow",
            AITool::Codex => "codex",
            AITool::Kilo => "kilo",
            AITool::Kiro => "kiro",
            AITool::Gemini => "gemini",
            AITool::Antigravity => "antigravity",
            AITool::Windsurf => "windsurf",
            AITool::Qoder => "qoder",
            AITool::CodeBuddy => "codebuddy",
            AITool::Copilot => "copilot",
        }
    }
}

impl fmt::Display for AITool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// TemplateDir enum
// ---------------------------------------------------------------------------

/// Template directory categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TemplateDir {
    Common,
    Claude,
    Cursor,
    OpenCode,
    IFlow,
    Codex,
    Kilo,
    Kiro,
    Gemini,
    Antigravity,
    Windsurf,
    Qoder,
    CodeBuddy,
    Copilot,
}

impl TemplateDir {
    /// The lowercase directory name on disk.
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateDir::Common => "common",
            TemplateDir::Claude => "claude",
            TemplateDir::Cursor => "cursor",
            TemplateDir::OpenCode => "opencode",
            TemplateDir::IFlow => "iflow",
            TemplateDir::Codex => "codex",
            TemplateDir::Kilo => "kilo",
            TemplateDir::Kiro => "kiro",
            TemplateDir::Gemini => "gemini",
            TemplateDir::Antigravity => "antigravity",
            TemplateDir::Windsurf => "windsurf",
            TemplateDir::Qoder => "qoder",
            TemplateDir::CodeBuddy => "codebuddy",
            TemplateDir::Copilot => "copilot",
        }
    }
}

impl fmt::Display for TemplateDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// CliFlag enum
// ---------------------------------------------------------------------------

/// CLI flag names for platform selection (e.g., `--claude`, `--cursor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CliFlag {
    Claude,
    Cursor,
    OpenCode,
    IFlow,
    Codex,
    Kilo,
    Kiro,
    Gemini,
    Antigravity,
    Windsurf,
    Qoder,
    CodeBuddy,
    Copilot,
}

impl CliFlag {
    /// The lowercase flag name (without leading dashes).
    pub fn as_str(&self) -> &'static str {
        match self {
            CliFlag::Claude => "claude",
            CliFlag::Cursor => "cursor",
            CliFlag::OpenCode => "opencode",
            CliFlag::IFlow => "iflow",
            CliFlag::Codex => "codex",
            CliFlag::Kilo => "kilo",
            CliFlag::Kiro => "kiro",
            CliFlag::Gemini => "gemini",
            CliFlag::Antigravity => "antigravity",
            CliFlag::Windsurf => "windsurf",
            CliFlag::Qoder => "qoder",
            CliFlag::CodeBuddy => "codebuddy",
            CliFlag::Copilot => "copilot",
        }
    }
}

impl fmt::Display for CliFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// AIToolConfig
// ---------------------------------------------------------------------------

/// Configuration for an AI tool.
#[derive(Clone, Debug)]
pub struct AIToolConfig {
    /// Display name of the tool.
    pub name: &'static str,
    /// Command template directory names to include.
    pub template_dirs: Vec<TemplateDir>,
    /// Config directory name in the project root (e.g., `.claude`).
    pub config_dir: &'static str,
    /// Whether the platform supports the shared `.agents/skills/` layer.
    pub supports_agent_skills: Option<bool>,
    /// Additional managed paths beyond `config_dir`.
    pub extra_managed_paths: Option<Vec<String>>,
    /// CLI flag name for `--flag` options.
    pub cli_flag: CliFlag,
    /// Whether this tool is checked by default in the interactive init prompt.
    pub default_checked: bool,
    /// Whether this tool uses Python hooks (affects Windows encoding detection).
    pub has_python_hooks: bool,
}

// ---------------------------------------------------------------------------
// Static registry
// ---------------------------------------------------------------------------

/// Lazily-initialised registry of all supported AI tools and their configurations.
///
/// This is the single source of truth for platform data.  When adding a new
/// platform, add an entry here and create:
///
/// 1. `src/configurators/{platform}.rs` -- configure function
/// 2. `src/templates/{platform}/`      -- template files
/// 3. Register in `src/configurators/mod.rs`
/// 4. Add CLI flag in `src/cli/mod.rs`
static AI_TOOLS: LazyLock<HashMap<AITool, AIToolConfig>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert(
        AITool::ClaudeCode,
        AIToolConfig {
            name: "Claude Code",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Claude],
            config_dir: ".claude",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Claude,
            default_checked: true,
            has_python_hooks: true,
        },
    );

    m.insert(
        AITool::Cursor,
        AIToolConfig {
            name: "Cursor",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Cursor],
            config_dir: ".cursor",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Cursor,
            default_checked: true,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::OpenCode,
        AIToolConfig {
            name: "OpenCode",
            template_dirs: vec![TemplateDir::Common, TemplateDir::OpenCode],
            config_dir: ".opencode",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::OpenCode,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::IFlow,
        AIToolConfig {
            name: "iFlow CLI",
            template_dirs: vec![TemplateDir::Common, TemplateDir::IFlow],
            config_dir: ".iflow",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::IFlow,
            default_checked: false,
            has_python_hooks: true,
        },
    );

    m.insert(
        AITool::Codex,
        AIToolConfig {
            name: "Codex",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Codex],
            config_dir: ".codex",
            supports_agent_skills: Some(true),
            extra_managed_paths: None,
            cli_flag: CliFlag::Codex,
            default_checked: false,
            has_python_hooks: true,
        },
    );

    m.insert(
        AITool::Kilo,
        AIToolConfig {
            name: "Kilo CLI",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Kilo],
            config_dir: ".kilocode",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Kilo,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::Kiro,
        AIToolConfig {
            name: "Kiro Code",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Kiro],
            config_dir: ".kiro/skills",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Kiro,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::Gemini,
        AIToolConfig {
            name: "Gemini CLI",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Gemini],
            config_dir: ".gemini",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Gemini,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::Antigravity,
        AIToolConfig {
            name: "Antigravity",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Antigravity],
            config_dir: ".agent/workflows",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Antigravity,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::Windsurf,
        AIToolConfig {
            name: "Windsurf",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Windsurf],
            config_dir: ".windsurf/workflows",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Windsurf,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::Qoder,
        AIToolConfig {
            name: "Qoder",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Qoder],
            config_dir: ".qoder",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::Qoder,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::CodeBuddy,
        AIToolConfig {
            name: "CodeBuddy",
            template_dirs: vec![TemplateDir::Common, TemplateDir::CodeBuddy],
            config_dir: ".codebuddy",
            supports_agent_skills: None,
            extra_managed_paths: None,
            cli_flag: CliFlag::CodeBuddy,
            default_checked: false,
            has_python_hooks: false,
        },
    );

    m.insert(
        AITool::Copilot,
        AIToolConfig {
            name: "GitHub Copilot",
            template_dirs: vec![TemplateDir::Common, TemplateDir::Copilot],
            config_dir: ".github/copilot",
            supports_agent_skills: None,
            extra_managed_paths: Some(vec![
                ".github/hooks".to_string(),
                ".github/prompts".to_string(),
            ]),
            cli_flag: CliFlag::Copilot,
            default_checked: false,
            has_python_hooks: true,
        },
    );

    m
});

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

/// Get the configuration for a specific AI tool.
pub fn get_tool_config(tool: AITool) -> &'static AIToolConfig {
    AI_TOOLS
        .get(&tool)
        .expect("All AITool variants must be present in AI_TOOLS")
}

/// Get all managed paths for a specific tool.
///
/// Always includes `config_dir`.  If the tool supports agent skills,
/// `.agents/skills` is appended.  Any `extra_managed_paths` are also included.
pub fn get_managed_paths(tool: AITool) -> Vec<&'static str> {
    let config = get_tool_config(tool);
    let mut paths: Vec<&str> = vec![config.config_dir];

    if config.supports_agent_skills == Some(true) {
        paths.push(".agents/skills");
    }

    if let Some(extra) = &config.extra_managed_paths {
        for p in extra {
            // SAFETY: The extra_managed_paths live in a LazyLock with 'static
            // lifetime, so the references are valid for the program's lifetime.
            paths.push(p.as_str());
        }
    }

    paths
}

/// Get template directories for a specific tool.
pub fn get_template_dirs(tool: AITool) -> &'static [TemplateDir] {
    &get_tool_config(tool).template_dirs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_present_in_registry() {
        for tool in AITool::all() {
            assert!(
                AI_TOOLS.contains_key(tool),
                "Missing AITool variant in registry: {tool}"
            );
        }
    }

    #[test]
    fn claude_code_config() {
        let cfg = get_tool_config(AITool::ClaudeCode);
        assert_eq!(cfg.name, "Claude Code");
        assert_eq!(cfg.config_dir, ".claude");
        assert!(cfg.default_checked);
        assert!(cfg.has_python_hooks);
    }

    #[test]
    fn copilot_managed_paths() {
        let paths = get_managed_paths(AITool::Copilot);
        assert!(paths.contains(&".github/copilot"));
        assert!(paths.contains(&".github/hooks"));
        assert!(paths.contains(&".github/prompts"));
    }

    #[test]
    fn codex_supports_agent_skills() {
        let paths = get_managed_paths(AITool::Codex);
        assert!(paths.contains(&".agents/skills"));
    }

    #[test]
    fn display_impl() {
        assert_eq!(AITool::ClaudeCode.to_string(), "claude-code");
        assert_eq!(AITool::CodeBuddy.to_string(), "codebuddy");
    }

    // --- Additional tests ported from TypeScript ---

    #[test]
    fn test_all_variants_count() {
        assert_eq!(AITool::all().len(), 13);
    }

    #[test]
    fn test_all_in_registry() {
        for tool in AITool::all() {
            assert!(
                AI_TOOLS.contains_key(tool),
                "AITool::{:?} missing from ai_tools() registry",
                tool
            );
        }
    }

    #[test]
    fn test_claude_code_config() {
        let cfg = get_tool_config(AITool::ClaudeCode);
        assert_eq!(cfg.name, "Claude Code");
        assert_eq!(cfg.cli_flag, CliFlag::Claude);
        assert!(cfg.default_checked);
    }

    #[test]
    fn test_copilot_managed_paths_extra() {
        let cfg = get_tool_config(AITool::Copilot);
        let extra = cfg.extra_managed_paths.as_ref().unwrap();
        assert!(extra.contains(&".github/hooks".to_string()));
        assert!(extra.contains(&".github/prompts".to_string()));
    }

    #[test]
    fn test_codex_agent_skills() {
        let cfg = get_tool_config(AITool::Codex);
        assert_eq!(cfg.supports_agent_skills, Some(true));
    }

    #[test]
    fn test_display_claude_code() {
        assert_eq!(AITool::ClaudeCode.to_string(), "claude-code");
    }

    #[test]
    fn test_unique_cli_flags() {
        let mut flags = std::collections::HashSet::new();
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            assert!(
                flags.insert(cfg.cli_flag),
                "Duplicate cli_flag: {:?} for {:?}",
                cfg.cli_flag,
                tool
            );
        }
    }

    #[test]
    fn test_unique_config_dirs() {
        let mut dirs = std::collections::HashSet::new();
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            assert!(
                dirs.insert(cfg.config_dir),
                "Duplicate config_dir: {} for {:?}",
                cfg.config_dir,
                tool
            );
        }
    }

    #[test]
    fn test_config_dirs_start_with_dot() {
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            assert!(
                cfg.config_dir.starts_with('.'),
                "config_dir for {:?} does not start with '.': {}",
                tool,
                cfg.config_dir
            );
        }
    }

    #[test]
    fn test_non_empty_names() {
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            assert!(!cfg.name.is_empty(), "Empty name for {:?}", tool);
        }
    }

    #[test]
    fn test_template_dirs_include_common() {
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            assert!(
                cfg.template_dirs.contains(&TemplateDir::Common),
                "template_dirs for {:?} does not include Common",
                tool
            );
        }
    }

    #[test]
    fn test_no_config_dir_collides_with_harness() {
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            assert_ne!(
                cfg.config_dir, ".harness-cli",
                "config_dir for {:?} should not be '.harness-cli'",
                tool
            );
        }
    }

    #[test]
    fn test_agent_skills_not_config_dir() {
        for tool in AITool::all() {
            let cfg = get_tool_config(*tool);
            if cfg.supports_agent_skills == Some(true) {
                assert_ne!(
                    cfg.config_dir, ".agents/skills",
                    "Platform {:?} with supportsAgentSkills should not use '.agents/skills' as configDir",
                    tool
                );
            }
        }
    }
}
