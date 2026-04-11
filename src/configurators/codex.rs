//! Codex configurator.
//!
//! Writes Codex templates to multiple directories:
//! - `.agents/skills/<name>/SKILL.md` -- shared skills
//! - `.codex/skills/<name>/SKILL.md`  -- Codex-specific skills
//! - `.codex/agents/<name>.toml`      -- custom agents
//! - `.codex/hooks/<name>`            -- hook scripts
//! - `.codex/hooks.json`              -- hooks config
//! - `.codex/config.toml`             -- project config

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::templates::codex as tmpl;
use crate::utils::file_writer::{ensure_dir, write_file};

use super::shared::resolve_placeholders;

/// Configure Codex by writing individual template files.
pub fn configure(cwd: &Path) -> Result<()> {
    // Shared skills -> .agents/skills/
    let shared_skills_root = cwd.join(".agents").join("skills");
    ensure_dir(&shared_skills_root)?;

    for skill in tmpl::get_all_skills() {
        let skill_dir = shared_skills_root.join(&skill.name);
        ensure_dir(&skill_dir)?;
        write_file(&skill_dir.join("SKILL.md"), &skill.content, false)?;
    }

    let codex_root = cwd.join(".codex");

    // Codex-specific skills -> .codex/skills/
    let codex_skills_root = codex_root.join("skills");
    ensure_dir(&codex_skills_root)?;

    for skill in tmpl::get_all_codex_skills() {
        let skill_dir = codex_skills_root.join(&skill.name);
        ensure_dir(&skill_dir)?;
        write_file(&skill_dir.join("SKILL.md"), &skill.content, false)?;
    }

    // Custom agents -> .codex/agents/
    let agents_root = codex_root.join("agents");
    ensure_dir(&agents_root)?;

    for agent in tmpl::get_all_agents() {
        write_file(
            &agents_root.join(format!("{}.toml", agent.name)),
            &agent.content,
            false,
        )?;
    }

    // Hooks -> .codex/hooks/
    let hooks_dir = codex_root.join("hooks");
    ensure_dir(&hooks_dir)?;

    for hook in tmpl::get_all_hooks() {
        write_file(&hooks_dir.join(&hook.name), &hook.content, false)?;
    }

    // Hooks config -> .codex/hooks.json
    write_file(
        &codex_root.join("hooks.json"),
        &resolve_placeholders(&tmpl::get_hooks_config()),
        false,
    )?;

    // Config -> .codex/config.toml
    let config = tmpl::get_config_template();
    write_file(
        &codex_root.join(&config.target_path),
        &config.content,
        false,
    )?;

    Ok(())
}

/// Collect template files for update tracking.
pub fn collect_templates() -> HashMap<String, String> {
    let mut files = HashMap::new();

    for skill in tmpl::get_all_skills() {
        files.insert(
            format!(".agents/skills/{}/SKILL.md", skill.name),
            skill.content,
        );
    }

    for skill in tmpl::get_all_codex_skills() {
        files.insert(
            format!(".codex/skills/{}/SKILL.md", skill.name),
            skill.content,
        );
    }

    for agent in tmpl::get_all_agents() {
        files.insert(format!(".codex/agents/{}.toml", agent.name), agent.content);
    }

    for hook in tmpl::get_all_hooks() {
        files.insert(format!(".codex/hooks/{}", hook.name), hook.content);
    }

    files.insert(
        ".codex/hooks.json".to_string(),
        resolve_placeholders(&tmpl::get_hooks_config()),
    );

    let config = tmpl::get_config_template();
    files.insert(format!(".codex/{}", config.target_path), config.content);

    files
}
