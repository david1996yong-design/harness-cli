//! GitHub Copilot configurator.
//!
//! Writes:
//! - `.github/prompts/*.prompt.md`          -- slash-command prompts
//! - `.github/copilot/hooks/session-start.py` -- hook scripts
//! - `.github/copilot/hooks.json`           -- hooks config (tracked copy)
//! - `.github/hooks/harness-cli.json`       -- VS Code Copilot discovery

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::copilot as tmpl;
use crate::utils::file_writer::{ensure_dir, write_file};

use super::shared::resolve_placeholders;

/// Configure GitHub Copilot by writing template files.
pub fn configure(cwd: &Path) -> Result<()> {
    let copilot_root = cwd.join(".github").join("copilot");

    // Prompt files -> .github/prompts/*.prompt.md
    let prompts_dir = cwd.join(".github").join("prompts");
    ensure_dir(&prompts_dir)?;

    for prompt in tmpl::get_all_prompts() {
        write_file(
            &prompts_dir.join(format!("{}.prompt.md", prompt.name)),
            &prompt.content,
            false,
        )?;
    }

    // Hook scripts -> .github/copilot/hooks/
    let hooks_dir = copilot_root.join("hooks");
    ensure_dir(&hooks_dir)?;

    for hook in tmpl::get_all_hooks() {
        write_file(&hooks_dir.join(&hook.name), &hook.content, false)?;
    }

    // Hooks config -> .github/copilot/hooks.json (tracked copy)
    let resolved_config = resolve_placeholders(&tmpl::get_hooks_config());
    write_file(&copilot_root.join("hooks.json"), &resolved_config, false)?;

    // Hooks config -> .github/hooks/harness-cli.json (VS Code discovery)
    let github_hooks_dir = cwd.join(".github").join("hooks");
    ensure_dir(&github_hooks_dir)?;
    write_file(
        &github_hooks_dir.join("harness-cli.json"),
        &resolved_config,
        false,
    )?;

    Ok(())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for prompt in tmpl::get_all_prompts() {
        files.insert(
            format!(".github/prompts/{}.prompt.md", prompt.name),
            prompt.content,
        );
    }

    for hook in tmpl::get_all_hooks() {
        files.insert(
            format!(".github/copilot/hooks/{}", hook.name),
            hook.content,
        );
    }

    let resolved_config = resolve_placeholders(&tmpl::get_hooks_config());
    files.insert(
        ".github/copilot/hooks.json".to_string(),
        resolved_config.clone(),
    );
    files.insert(
        ".github/hooks/harness-cli.json".to_string(),
        resolved_config,
    );

    files
}
