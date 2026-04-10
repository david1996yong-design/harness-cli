//! Init command -- set up Harness CLI in the current project.
//!
//! Ported from `packages/cli/src/commands/init.ts`.

use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{Confirm, Input, MultiSelect, Select};

use crate::configurators;
use crate::constants::paths::{constructed, dir_names, file_names};
use crate::constants::version::VERSION;
use crate::types::ai_tools::{get_tool_config, AITool};
use crate::utils::file_writer::{ensure_dir, set_write_mode, write_file, WriteMode};
use crate::utils::project_detector::{
    detect_monorepo, detect_project_type, sanitize_pkg_name, DetectedPackage, ProjectType,
};
use crate::utils::proxy::{mask_proxy_url, setup_proxy};
use crate::utils::template_fetcher::{
    download_registry_direct, download_template_by_id, fetch_template_index,
    parse_registry_source, probe_registry_index, RegistrySource, SpecTemplate, TemplateStrategy,
    TEMPLATE_INDEX_URL,
};
use crate::utils::template_hash::initialize_hashes;

// =============================================================================
// InitOptions
// =============================================================================

/// Options for the `init` command.
pub struct InitOptions {
    pub cursor: bool,
    pub claude: bool,
    pub iflow: bool,
    pub opencode: bool,
    pub codex: bool,
    pub kilo: bool,
    pub kiro: bool,
    pub gemini: bool,
    pub antigravity: bool,
    pub windsurf: bool,
    pub qoder: bool,
    pub codebuddy: bool,
    pub copilot: bool,
    pub yes: bool,
    pub user: Option<String>,
    pub force: bool,
    pub skip_existing: bool,
    pub template: Option<String>,
    pub overwrite: bool,
    pub append: bool,
    pub registry: Option<String>,
    /// `None` = auto-detect, `Some(true)` = force monorepo, `Some(false)` = skip monorepo
    pub monorepo: Option<bool>,
}

/// Check if a tool flag was set on `InitOptions`.
fn is_tool_flag_set(options: &InitOptions, key: &str) -> bool {
    match key {
        "claude" => options.claude,
        "cursor" => options.cursor,
        "opencode" => options.opencode,
        "iflow" => options.iflow,
        "codex" => options.codex,
        "kilo" => options.kilo,
        "kiro" => options.kiro,
        "gemini" => options.gemini,
        "antigravity" => options.antigravity,
        "windsurf" => options.windsurf,
        "qoder" => options.qoder,
        "codebuddy" => options.codebuddy,
        "copilot" => options.copilot,
        _ => false,
    }
}

// =============================================================================
// Python command detection
// =============================================================================

fn get_python_command() -> String {
    if std::process::Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return "python3".to_string();
    }
    if std::process::Command::new("python")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return "python".to_string();
    }
    "python3".to_string()
}

// =============================================================================
// Bootstrap task
// =============================================================================

const BOOTSTRAP_TASK_NAME: &str = "00-bootstrap-guidelines";

fn get_bootstrap_prd_content(
    project_type: ProjectType,
    packages: Option<&[DetectedPackage]>,
) -> String {
    let header = r#"# Bootstrap: Fill Project Development Guidelines

## Purpose

Welcome to Harness CLI! This is your first task.

AI agents use `.harness-cli/spec/` to understand YOUR project's coding conventions.
**Starting from scratch = AI writes generic code that doesn't match your project style.**

Filling these guidelines is a one-time setup that pays off for every future AI session.

---

## Your Task

Fill in the guideline files based on your **existing codebase**.
"#;

    let backend_section = r#"

### Backend Guidelines

| File | What to Document |
|------|------------------|
| `.harness-cli/spec/backend/directory-structure.md` | Where different file types go (routes, services, utils) |
| `.harness-cli/spec/backend/database-guidelines.md` | ORM, migrations, query patterns, naming conventions |
| `.harness-cli/spec/backend/error-handling.md` | How errors are caught, logged, and returned |
| `.harness-cli/spec/backend/logging-guidelines.md` | Log levels, format, what to log |
| `.harness-cli/spec/backend/quality-guidelines.md` | Code review standards, testing requirements |
"#;

    let frontend_section = r#"

### Frontend Guidelines

| File | What to Document |
|------|------------------|
| `.harness-cli/spec/frontend/directory-structure.md` | Component/page/hook organization |
| `.harness-cli/spec/frontend/component-guidelines.md` | Component patterns, props conventions |
| `.harness-cli/spec/frontend/hook-guidelines.md` | Custom hook naming, patterns |
| `.harness-cli/spec/frontend/state-management.md` | State library, patterns, what goes where |
| `.harness-cli/spec/frontend/type-safety.md` | TypeScript conventions, type organization |
| `.harness-cli/spec/frontend/quality-guidelines.md` | Linting, testing, accessibility |
"#;

    let footer = r#"

### Thinking Guides (Optional)

The `.harness-cli/spec/guides/` directory contains thinking guides that are already
filled with general best practices. You can customize them for your project if needed.

---

## How to Fill Guidelines

### Step 0: Import from Existing Specs (Recommended)

Many projects already have coding conventions documented. **Check these first** before writing from scratch:

| File / Directory | Tool |
|------|------|
| `CLAUDE.md` / `CLAUDE.local.md` | Claude Code |
| `AGENTS.md` | Codex / Claude Code / agent-compatible tools |
| `.cursorrules` | Cursor |
| `.cursor/rules/*.mdc` | Cursor (rules directory) |
| `.windsurfrules` | Windsurf |
| `.clinerules` | Cline |
| `.roomodes` | Roo Code |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `.vscode/settings.json` -> `github.copilot.chat.codeGeneration.instructions` | VS Code Copilot |
| `CONVENTIONS.md` / `.aider.conf.yml` | aider |
| `CONTRIBUTING.md` | General project conventions |
| `.editorconfig` | Editor formatting rules |

If any of these exist, read them first and extract the relevant coding conventions into the corresponding `.harness-cli/spec/` files. This saves significant effort compared to writing everything from scratch.

### Step 1: Analyze the Codebase

Ask AI to help discover patterns from actual code:

- "Read all existing config files (CLAUDE.md, .cursorrules, etc.) and extract coding conventions into .harness-cli/spec/"
- "Analyze my codebase and document the patterns you see"
- "Find error handling / component / API patterns and document them"

### Step 2: Document Reality, Not Ideals

Write what your codebase **actually does**, not what you wish it did.
AI needs to match existing patterns, not introduce new ones.

- **Look at existing code** - Find 2-3 examples of each pattern
- **Include file paths** - Reference real files as examples
- **List anti-patterns** - What does your team avoid?

---

## Completion Checklist

- [ ] Guidelines filled for your project type
- [ ] At least 2-3 real code examples in each guideline
- [ ] Anti-patterns documented

When done:

```bash
python3 ./.harness-cli/scripts/task.py finish
python3 ./.harness-cli/scripts/task.py archive 00-bootstrap-guidelines
```

---

## Why This Matters

After completing this task:

1. AI will write code that matches your project style
2. Relevant `/hc:before-*-dev` commands will inject real context
3. `/hc:check-*` commands will validate against your actual standards
4. Future developers (human or AI) will onboard faster
"#;

    let mut content = header.to_string();

    if let Some(pkgs) = packages {
        if !pkgs.is_empty() {
            for pkg in pkgs {
                let pkg_type = if pkg.type_ == ProjectType::Unknown {
                    "fullstack"
                } else {
                    match pkg.type_ {
                        ProjectType::Frontend => "frontend",
                        ProjectType::Backend => "backend",
                        ProjectType::Fullstack => "fullstack",
                        ProjectType::Unknown => "fullstack",
                    }
                };
                let spec_name = sanitize_pkg_name(&pkg.name);
                content += &format!("\n### Package: {} (`spec/{}/`)\n", pkg.name, spec_name);
                if pkg_type != "frontend" {
                    content += &format!(
                        "\n- Backend guidelines: `.harness-cli/spec/{}/backend/`\n",
                        spec_name
                    );
                }
                if pkg_type != "backend" {
                    content += &format!(
                        "\n- Frontend guidelines: `.harness-cli/spec/{}/frontend/`\n",
                        spec_name
                    );
                }
            }
        }
    } else {
        match project_type {
            ProjectType::Frontend => content += frontend_section,
            ProjectType::Backend => content += backend_section,
            _ => {
                // fullstack or unknown
                content += backend_section;
                content += frontend_section;
            }
        }
    }
    content += footer;

    content
}

fn get_bootstrap_task_json(
    developer: &str,
    project_type: ProjectType,
    packages: Option<&[DetectedPackage]>,
) -> serde_json::Value {
    let today = chrono_today();

    let (subtasks, related_files) = if let Some(pkgs) = packages {
        if !pkgs.is_empty() {
            let mut st: Vec<serde_json::Value> = pkgs
                .iter()
                .map(|pkg| {
                    serde_json::json!({
                        "name": format!("Fill guidelines for {}", pkg.name),
                        "status": "pending"
                    })
                })
                .collect();
            st.push(serde_json::json!({"name": "Add code examples", "status": "pending"}));

            let rf: Vec<String> = pkgs
                .iter()
                .map(|pkg| format!(".harness-cli/spec/{}/", sanitize_pkg_name(&pkg.name)))
                .collect();
            (st, rf)
        } else {
            default_subtasks_and_files(project_type)
        }
    } else {
        default_subtasks_and_files(project_type)
    };

    let pt_str = match project_type {
        ProjectType::Frontend => "frontend",
        ProjectType::Backend => "backend",
        ProjectType::Fullstack => "fullstack",
        ProjectType::Unknown => "fullstack",
    };

    serde_json::json!({
        "id": BOOTSTRAP_TASK_NAME,
        "name": "Bootstrap Guidelines",
        "description": "Fill in project development guidelines for AI agents",
        "status": "in_progress",
        "dev_type": "docs",
        "priority": "P1",
        "creator": developer,
        "assignee": developer,
        "createdAt": today,
        "completedAt": null,
        "commit": null,
        "subtasks": subtasks,
        "children": [],
        "parent": null,
        "relatedFiles": related_files,
        "notes": format!("First-time setup task created by harness-cli init ({} project)", pt_str),
        "meta": {}
    })
}

fn default_subtasks_and_files(
    project_type: ProjectType,
) -> (Vec<serde_json::Value>, Vec<String>) {
    match project_type {
        ProjectType::Frontend => (
            vec![
                serde_json::json!({"name": "Fill frontend guidelines", "status": "pending"}),
                serde_json::json!({"name": "Add code examples", "status": "pending"}),
            ],
            vec![".harness-cli/spec/frontend/".to_string()],
        ),
        ProjectType::Backend => (
            vec![
                serde_json::json!({"name": "Fill backend guidelines", "status": "pending"}),
                serde_json::json!({"name": "Add code examples", "status": "pending"}),
            ],
            vec![".harness-cli/spec/backend/".to_string()],
        ),
        _ => (
            vec![
                serde_json::json!({"name": "Fill backend guidelines", "status": "pending"}),
                serde_json::json!({"name": "Fill frontend guidelines", "status": "pending"}),
                serde_json::json!({"name": "Add code examples", "status": "pending"}),
            ],
            vec![
                ".harness-cli/spec/backend/".to_string(),
                ".harness-cli/spec/frontend/".to_string(),
            ],
        ),
    }
}

/// Simple ISO date string (YYYY-MM-DD) without pulling in chrono crate.
fn chrono_today() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day) = chrono_today_parts(secs);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Convert unix timestamp to (year, month, day). Public for use by update command.
pub fn chrono_today_parts(secs: u64) -> (u64, u64, u64) {
    let days = secs / 86400;
    days_to_ymd(days)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Adapted from civil_from_days algorithm
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}

fn create_bootstrap_task(
    cwd: &Path,
    developer: &str,
    project_type: ProjectType,
    packages: Option<&[DetectedPackage]>,
) -> bool {
    let task_dir = cwd
        .join(constructed::TASKS)
        .join(BOOTSTRAP_TASK_NAME);
    let task_relative_path = format!("{}/{}", constructed::TASKS, BOOTSTRAP_TASK_NAME);

    if task_dir.exists() {
        return true;
    }

    if std::fs::create_dir_all(&task_dir).is_err() {
        return false;
    }

    // Write task.json
    let task_json = get_bootstrap_task_json(developer, project_type, packages);
    let task_json_path = task_dir.join(file_names::TASK_JSON);
    if let Ok(json_str) = serde_json::to_string_pretty(&task_json) {
        if std::fs::write(&task_json_path, json_str).is_err() {
            return false;
        }
    }

    // Write prd.md
    let prd_content = get_bootstrap_prd_content(project_type, packages);
    let prd_path = task_dir.join(file_names::PRD);
    if std::fs::write(&prd_path, prd_content).is_err() {
        return false;
    }

    // Set as current task
    let current_task_file = cwd.join(constructed::CURRENT_TASK_FILE);
    let _ = std::fs::write(current_task_file, task_relative_path);

    true
}

// =============================================================================
// Monorepo config
// =============================================================================

fn write_monorepo_config(cwd: &Path, packages: &[DetectedPackage]) {
    let config_path = cwd.join(dir_names::WORKFLOW).join("config.yaml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return, // Config not created yet
    };

    // Don't overwrite if packages: already exists (re-init case)
    if regex::Regex::new(r"(?m)^packages\s*:")
        .unwrap()
        .is_match(&content)
    {
        return;
    }

    let mut lines = vec![
        String::new(),
        "# Auto-detected monorepo packages".to_string(),
        "packages:".to_string(),
    ];

    for pkg in packages {
        lines.push(format!("  {}:", sanitize_pkg_name(&pkg.name)));
        lines.push(format!("    path: {}", pkg.path));
        if pkg.is_submodule {
            lines.push("    type: submodule".to_string());
        }
    }

    let default_pkg = packages
        .iter()
        .find(|p| !p.is_submodule)
        .or(packages.first())
        .map(|p| p.name.clone());

    if let Some(name) = default_pkg {
        lines.push(format!("default_package: {}", name));
    }

    let new_content = format!("{}\n{}\n", content.trim_end(), lines.join("\n"));
    let _ = std::fs::write(&config_path, new_content);
}

// =============================================================================
// What We Solve
// =============================================================================

fn print_what_we_solve() {
    println!(
        "{}{}",
        "Sound familiar? ".dimmed(),
        "You'll never say these again!!".bold()
    );
    println!();

    let pain_points = [
        (
            "\"Fix A -> break B -> fix B -> break A...\"",
            "Thinking Guides + Ralph Loop: Think first, verify after",
        ),
        (
            "\"Wrote CLAUDE.md, AI ignored it. Reminded AI, it forgot 5 turns later.\"",
            "Spec Injection: Rules enforced per task, not per chat",
        ),
        (
            "\"Code works but nothing connects...\"",
            "Cross-Layer Guide: Map data flow before coding",
        ),
        (
            "\"Asked for a button, got 9000 lines\"",
            "Plan Agent: Rejects and splits oversized tasks",
        ),
    ];

    for (pain, solution) in &pain_points {
        println!("{} {}", "x ".dimmed(), pain);
        println!("  {} {}", "v".green(), solution.white());
    }

    println!();
}

// =============================================================================
// Root files
// =============================================================================

fn create_root_files(cwd: &Path) -> Result<()> {
    let agents_path = cwd.join("AGENTS.md");

    // Try to get AGENTS.md content from embedded templates
    let agents_content =
        crate::templates::extract::get_embedded_file::<crate::templates::extract::MarkdownTemplates>(
            "AGENTS.md",
        )
        .unwrap_or_else(|| {
            // Fallback minimal content
            "# AGENTS.md\n\nSee `.harness-cli/` for project workflow and guidelines.\n".to_string()
        });

    let written = write_file(&agents_path, &agents_content, false)?;
    if written {
        println!("{}", "  Created AGENTS.md".blue());
    }
    Ok(())
}

// =============================================================================
// Main init function
// =============================================================================

/// Run the `init` command.
pub fn init(options: InitOptions) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Generate ASCII art banner
    let font = figlet_rs::FIGfont::standard().unwrap_or_else(|_| {
        // Fallback: try to load Rebel font, or use standard
        figlet_rs::FIGfont::standard().unwrap()
    });
    let banner = font
        .convert("Harness CLI")
        .map(|f| f.to_string())
        .unwrap_or_else(|| "Harness CLI".to_string());
    println!("{}", format!("\n{}", banner.trim_end()).cyan());
    println!(
        "{}",
        "\n   All-in-one AI framework & toolkit for Claude Code & Cursor\n".dimmed()
    );

    // Set up proxy before any network calls
    let proxy_url = setup_proxy();
    if let Some(ref url) = proxy_url {
        println!("{}", format!("   Using proxy: {}\n", mask_proxy_url(url)).dimmed());
    }

    // Set write mode based on options
    if options.force {
        set_write_mode(WriteMode::Force);
        println!("{}", "Mode: Force overwrite existing files\n".dimmed());
    } else if options.skip_existing {
        set_write_mode(WriteMode::Skip);
        println!("{}", "Mode: Skip existing files\n".dimmed());
    } else {
        set_write_mode(WriteMode::Ask);
    }

    // Detect developer name from git config or options
    let mut developer_name = options.user.clone();
    if developer_name.is_none() {
        let is_git_repo = cwd.join(".git").exists();
        if is_git_repo {
            if let Ok(output) = std::process::Command::new("git")
                .args(["config", "user.name"])
                .current_dir(&cwd)
                .output()
            {
                if output.status.success() {
                    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !name.is_empty() {
                        developer_name = Some(name);
                    }
                }
            }
        }
    }

    if let Some(ref name) = developer_name {
        println!("{} {}", "  Developer:".blue(), name.dimmed());
    } else if !options.yes {
        println!(
            "{}",
            format!(
                "\nHarness CLI supports team collaboration - each developer has their own\n\
                 workspace directory ({}/{{name}}/) to track AI sessions.\n\
                 Tip: Usually this is your git username (git config user.name).\n",
                constructed::WORKSPACE
            )
            .dimmed()
        );

        loop {
            let name: String = Input::new()
                .with_prompt("Your name")
                .interact_text()
                .unwrap_or_default();
            if name.is_empty() {
                println!("{}", "Name is required".yellow());
                continue;
            }
            developer_name = Some(name.clone());
            println!("{} {}", "  Developer:".blue(), name.dimmed());
            break;
        }
    }

    // Detect project type (silent - no output)
    let detected_type = detect_project_type(&cwd);

    // Parse custom registry source
    let mut registry: Option<RegistrySource> = None;
    if let Some(ref reg_source) = options.registry {
        match parse_registry_source(reg_source) {
            Ok(r) => registry = Some(r),
            Err(e) => {
                println!("{}", format!("  {}", e).red());
                return Ok(());
            }
        }
    }

    // Determine template strategy from flags
    let mut template_strategy = if options.overwrite {
        TemplateStrategy::Overwrite
    } else if options.append {
        TemplateStrategy::Append
    } else {
        TemplateStrategy::Skip
    };

    // =========================================================================
    // Monorepo Detection
    // =========================================================================

    let mut monorepo_packages: Option<Vec<DetectedPackage>> = None;
    let mut _remote_spec_packages: std::collections::HashSet<String> = std::collections::HashSet::new();

    if options.monorepo != Some(false) {
        let detected = detect_monorepo(&cwd);

        if options.monorepo == Some(true) && detected.is_none() {
            println!(
                "{}",
                "Error: --monorepo specified but no monorepo configuration found.".red()
            );
            return Ok(());
        }

        if let Some(ref packages) = detected {
            if !packages.is_empty() {
                let enable_monorepo = if options.monorepo == Some(true) || options.yes {
                    true
                } else {
                    // Show detected packages and ask
                    println!("{}", "\n  Detected monorepo packages:".blue());
                    for pkg in packages {
                        let sub = if pkg.is_submodule {
                            " (submodule)".dimmed().to_string()
                        } else {
                            String::new()
                        };
                        let type_str = match pkg.type_ {
                            ProjectType::Frontend => "frontend",
                            ProjectType::Backend => "backend",
                            ProjectType::Fullstack => "fullstack",
                            ProjectType::Unknown => "unknown",
                        };
                        println!(
                            "{}",
                            format!(
                                "   - {} ({}) [{}]{}",
                                pkg.name, pkg.path, type_str, sub
                            )
                            .dimmed()
                        );
                    }
                    println!();

                    Confirm::new()
                        .with_prompt("Enable monorepo mode?")
                        .default(true)
                        .interact()
                        .unwrap_or(false)
                };

                if enable_monorepo {
                    // Per-package template selection
                    if !options.yes && options.template.is_none() {
                        for pkg in packages {
                            let choices = &["From scratch (Harness CLI default)", "Download remote template"];
                            let selection = Select::new()
                                .with_prompt(format!("Spec source for {} ({}):", pkg.name, pkg.path))
                                .items(choices)
                                .default(0)
                                .interact()
                                .unwrap_or(0);

                            if selection == 1 {
                                // Download remote template for this package
                                let dest_dir = cwd
                                    .join(constructed::SPEC)
                                    .join(sanitize_pkg_name(&pkg.name));
                                println!("{}", format!("  Select template for {}...", pkg.name).blue());

                                let templates = fetch_template_index(None);
                                let spec_templates: Vec<&SpecTemplate> = templates
                                    .iter()
                                    .filter(|t| t.type_ == "spec")
                                    .collect();

                                if !spec_templates.is_empty() {
                                    let template_names: Vec<String> = spec_templates
                                        .iter()
                                        .map(|t| format!("{} ({})", t.id, t.name))
                                        .collect();
                                    let sel = Select::new()
                                        .with_prompt(format!("Select template for {}:", pkg.name))
                                        .items(&template_names)
                                        .interact()
                                        .unwrap_or(0);

                                    let selected = spec_templates[sel];
                                    let result = download_template_by_id(
                                        &cwd,
                                        &selected.id,
                                        template_strategy,
                                        Some(selected),
                                        registry.as_ref(),
                                        Some(&dest_dir),
                                    );

                                    if result.success {
                                        println!("{}", format!("   {}", result.message).green());
                                        _remote_spec_packages
                                            .insert(sanitize_pkg_name(&pkg.name));
                                    } else {
                                        println!("{}", format!("   {}", result.message).yellow());
                                        println!(
                                            "{}",
                                            "   Falling back to blank spec...".dimmed()
                                        );
                                    }
                                } else {
                                    println!(
                                        "{}",
                                        "   No templates available. Using blank spec.".dimmed()
                                    );
                                }
                            }
                        }
                    } else if let Some(ref tmpl) = options.template {
                        // --template as default for all packages
                        for pkg in packages {
                            let dest_dir = cwd
                                .join(constructed::SPEC)
                                .join(sanitize_pkg_name(&pkg.name));
                            let result = download_template_by_id(
                                &cwd,
                                tmpl,
                                template_strategy,
                                None,
                                registry.as_ref(),
                                Some(&dest_dir),
                            );
                            if result.success && !result.skipped {
                                _remote_spec_packages.insert(sanitize_pkg_name(&pkg.name));
                            }
                        }
                    }

                    monorepo_packages = Some(packages.clone());
                }
            }
        }
    }

    // =========================================================================
    // Tool selection
    // =========================================================================

    let tool_choices = configurators::get_init_tool_choices();
    let explicit_tools: Vec<AITool> = tool_choices
        .iter()
        .filter(|t| is_tool_flag_set(&options, t.key.as_str()))
        .map(|t| t.platform_id)
        .collect();

    let selected_tools: Vec<AITool> = if !explicit_tools.is_empty() {
        explicit_tools
    } else if options.yes {
        tool_choices
            .iter()
            .filter(|t| t.default_checked)
            .map(|t| t.platform_id)
            .collect()
    } else {
        // Interactive mode
        let display_names: Vec<&str> = tool_choices.iter().map(|t| t.name).collect();
        let defaults: Vec<bool> = tool_choices.iter().map(|t| t.default_checked).collect();

        let selected_indices = MultiSelect::new()
            .with_prompt("Select AI tools to configure")
            .items(&display_names)
            .defaults(&defaults)
            .interact()
            .unwrap_or_default();

        selected_indices
            .iter()
            .map(|&i| tool_choices[i].platform_id)
            .collect()
    };

    // Treat unknown project type as fullstack
    let project_type = if detected_type == ProjectType::Unknown {
        ProjectType::Fullstack
    } else {
        detected_type
    };

    if selected_tools.is_empty() {
        println!(
            "{}",
            "No tools selected. At least one tool is required.".yellow()
        );
        return Ok(());
    }


    // =========================================================================
    // Template Selection (single-repo only)
    // =========================================================================

    let mut selected_template: Option<String> = None;
    let mut fetched_templates: Vec<SpecTemplate> = Vec::new();

    let index_url = registry
        .as_ref()
        .map(|r| format!("{}/index.json", r.raw_base_url))
        .unwrap_or_else(|| TEMPLATE_INDEX_URL.to_string());

    if monorepo_packages.is_some() {
        // Template selection already handled above for monorepo
    } else if let Some(ref tmpl) = options.template {
        selected_template = Some(tmpl.clone());
    } else if !options.yes {
        // Interactive template selection
        println!(
            "{}",
            format!(
                "   Fetching available templates from {}",
                registry
                    .as_ref()
                    .map(|r| r.giget_source.as_str())
                    .unwrap_or(TEMPLATE_INDEX_URL)
            )
            .dimmed()
        );

        let (templates, registry_probe_not_found) = if let Some(ref _reg) = registry {
            let (templates, is_not_found) = probe_registry_index(&index_url);
            (templates, is_not_found)
        } else {
            (fetch_template_index(Some(&index_url)), false)
        };

        fetched_templates = templates.clone();

        if templates.is_empty() && registry.is_some() && registry_probe_not_found {
            println!(
                "{}",
                "   No index.json found at registry. Will download as direct spec template."
                    .dimmed()
            );
        } else if templates.is_empty() && registry.is_some() {
            println!(
                "{}",
                "   Could not reach registry (network issue). Check your connection and try again."
                    .red()
            );
            return Ok(());
        } else if templates.is_empty() {
            println!(
                "{}",
                "   Could not fetch templates (offline or server unavailable).".dimmed()
            );
            println!("{}", "   Using blank templates.\n".dimmed());
        }

        if !templates.is_empty() {
            let spec_templates: Vec<&SpecTemplate> =
                templates.iter().filter(|t| t.type_ == "spec").collect();

            let mut template_choices: Vec<String> = Vec::new();
            let mut template_ids: Vec<String> = Vec::new();

            if registry.is_none() {
                template_choices.push("from scratch (default)".to_string());
                template_ids.push("blank".to_string());
            }

            for t in &spec_templates {
                template_choices.push(format!("{} ({})", t.id, t.name));
                template_ids.push(t.id.clone());
            }

            if registry.is_none() {
                template_choices.push("custom (enter a registry source)".to_string());
                template_ids.push("__custom__".to_string());
            }

            if !template_choices.is_empty() {
                let sel = Select::new()
                    .with_prompt("Select a spec template")
                    .items(&template_choices)
                    .default(0)
                    .interact()
                    .unwrap_or(0);

                let chosen_id = &template_ids[sel];

                if chosen_id == "__custom__" {
                    let custom_source: String = Input::new()
                        .with_prompt(
                            "Enter registry source (e.g., gh:myorg/myrepo/specs), or press Enter to skip",
                        )
                        .allow_empty(true)
                        .interact_text()
                        .unwrap_or_default();

                    if !custom_source.is_empty() {
                        match parse_registry_source(&custom_source) {
                            Ok(r) => {
                                registry = Some(r);
                            }
                            Err(e) => {
                                println!("{}", format!("   {}", e).red());
                            }
                        }
                    }
                } else if chosen_id != "blank" {
                    selected_template = Some(chosen_id.clone());

                    // Check if spec directory exists
                    let spec_dir = cwd.join(constructed::SPEC);
                    if spec_dir.exists() && !options.overwrite && !options.append {
                        let action_choices = &[
                            "Skip (keep existing)",
                            "Overwrite (replace all)",
                            "Append (add missing files only)",
                        ];
                        let action_sel = Select::new()
                            .with_prompt(format!(
                                "Directory {} already exists. What do you want to do?",
                                constructed::SPEC
                            ))
                            .items(action_choices)
                            .default(0)
                            .interact()
                            .unwrap_or(0);

                        template_strategy = match action_sel {
                            1 => TemplateStrategy::Overwrite,
                            2 => TemplateStrategy::Append,
                            _ => TemplateStrategy::Skip,
                        };
                    }
                }
            }
        }
    }

    // -y mode with --registry (no --template): probe index.json
    if options.yes && selected_template.is_none() && monorepo_packages.is_none() {
        if let Some(ref reg) = registry {
            let probe_url = format!("{}/index.json", reg.raw_base_url);
            let (probe_templates, is_not_found) = probe_registry_index(&probe_url);

            if !probe_templates.is_empty() {
                println!(
                    "{}",
                    "Error: Registry is a marketplace with multiple templates. \
                     Use --template <id> to specify which one, or remove -y for interactive selection."
                        .red()
                );
                return Ok(());
            }
            if !is_not_found {
                println!(
                    "{}",
                    "Error: Could not reach registry (network issue). Check your connection and try again."
                        .red()
                );
                return Ok(());
            }
        }
    }

    // =========================================================================
    // Download Remote Template (if selected)
    // =========================================================================

    let mut _use_remote_template = false;

    if let Some(ref tmpl_id) = selected_template {
        println!(
            "{}",
            format!("  Downloading template \"{}\"...", tmpl_id).blue()
        );
        println!(
            "{}",
            "   This may take a moment on slow connections.".dimmed()
        );

        let prefetched = fetched_templates.iter().find(|t| t.id == *tmpl_id);

        let result = download_template_by_id(
            &cwd,
            tmpl_id,
            template_strategy,
            prefetched,
            registry.as_ref(),
            None,
        );

        if result.success {
            if result.skipped {
                println!("{}", format!("   {}", result.message).dimmed());
            } else {
                println!("{}", format!("   {}", result.message).green());
                _use_remote_template = true;
            }
        } else {
            println!("{}", format!("   {}", result.message).yellow());
            println!("{}", "   Falling back to blank templates...".dimmed());
            let retry_cmd = if let Some(ref reg) = registry {
                format!(
                    "harness-cli init --registry {} --template {}",
                    reg.giget_source, tmpl_id
                )
            } else {
                format!("harness-cli init --template {}", tmpl_id)
            };
            println!("{}", format!("   You can retry later: {}", retry_cmd).dimmed());
        }
    } else if let Some(ref reg) = registry {
        if fetched_templates.is_empty() && monorepo_packages.is_none() {
        // Direct download mode
        println!(
            "{}",
            format!("  Downloading spec from {}...", reg.giget_source).blue()
        );
        println!(
            "{}",
            "   This may take a moment on slow connections.".dimmed()
        );

        // Ask about existing spec dir in interactive mode
        if !options.yes && !options.overwrite && !options.append {
            let spec_dir = cwd.join(constructed::SPEC);
            if spec_dir.exists() {
                let action_choices = &[
                    "Skip (keep existing)",
                    "Overwrite (replace all)",
                    "Append (add missing files only)",
                ];
                let action_sel = Select::new()
                    .with_prompt(format!(
                        "Directory {} already exists. What do you want to do?",
                        constructed::SPEC
                    ))
                    .items(action_choices)
                    .default(0)
                    .interact()
                    .unwrap_or(0);

                template_strategy = match action_sel {
                    1 => TemplateStrategy::Overwrite,
                    2 => TemplateStrategy::Append,
                    _ => TemplateStrategy::Skip,
                };
            }
        }

        let result = download_registry_direct(&cwd, reg, template_strategy, None);
        if result.success {
            if result.skipped {
                println!("{}", format!("   {}", result.message).dimmed());
            } else {
                println!("{}", format!("   {}", result.message).green());
                _use_remote_template = true;
            }
        } else {
            println!("{}", format!("   {}", result.message).yellow());
            println!("{}", "   Falling back to blank templates...".dimmed());
            println!(
                "{}",
                format!(
                    "   You can retry later: harness-cli init --registry {}",
                    reg.giget_source
                )
                .dimmed()
            );
        }
        } // end if fetched_templates.is_empty()
    }

    // =========================================================================
    // Create Workflow Structure
    // =========================================================================

    println!("{}", "  Creating workflow structure...".blue());

    // TODO: Call configurators::workflow::create_workflow_structure when available.
    // For now, ensure the basic directory structure exists.
    let workflow_dir = cwd.join(dir_names::WORKFLOW);
    ensure_dir(&workflow_dir)?;
    ensure_dir(&cwd.join(constructed::WORKSPACE))?;
    ensure_dir(&cwd.join(constructed::TASKS))?;
    ensure_dir(&cwd.join(constructed::SPEC))?;
    ensure_dir(&cwd.join(constructed::SCRIPTS))?;
    println!(
        "{}",
        "  (Workflow structure: basic directories created)".dimmed()
    );

    // Write monorepo packages to config.yaml
    if let Some(ref pkgs) = monorepo_packages {
        write_monorepo_config(&cwd, pkgs);
        println!("{}", "  Monorepo packages written to config.yaml".blue());
    }

    // Write version file
    let version_path = cwd.join(dir_names::WORKFLOW).join(".version");
    std::fs::write(&version_path, VERSION)?;

    // Configure selected tools by copying entire directories
    for &tool_id in &selected_tools {
        let cfg = get_tool_config(tool_id);
        println!("{}", format!("  Configuring {}...", cfg.name).blue());
        configurators::configure_platform(tool_id, &cwd)?;
    }

    // Show Windows platform detection notice
    #[cfg(target_os = "windows")]
    {
        let has_python_platform = selected_tools
            .iter()
            .any(|&id| get_tool_config(id).has_python_hooks);
        if has_python_platform {
            println!(
                "{}",
                "  Windows detected: Using \"python\" for hooks".yellow()
            );
        }
    }

    // Create root files (AGENTS.md)
    create_root_files(&cwd)?;

    // Initialize template hashes for modification tracking
    let all_managed = configurators::all_managed_dirs();
    let all_managed_refs: Vec<&str> = all_managed.iter().map(|s| s.as_str()).collect();
    let hashed_count = initialize_hashes(&cwd, &all_managed_refs);
    if hashed_count > 0 {
        println!(
            "{}",
            format!("  Tracking {} template files for updates", hashed_count).dimmed()
        );
    }

    // Initialize developer identity
    if let Some(ref name) = developer_name {
        let python_cmd = get_python_command();
        let script_path = cwd.join(constructed::SCRIPTS).join("init_developer.py");
        if script_path.exists() {
            let _ = std::process::Command::new(&python_cmd)
                .arg(&script_path)
                .arg(name)
                .current_dir(&cwd)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }

        // Create bootstrap task
        create_bootstrap_task(
            &cwd,
            name,
            project_type,
            monorepo_packages.as_deref(),
        );
    }

    // Print "What We Solve" section
    println!();
    print_what_we_solve();

    Ok(())
}
