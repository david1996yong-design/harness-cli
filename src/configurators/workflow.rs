//! Workflow structure creation.
//!
//! Creates the `.harness-cli/` directory tree for a new project, including
//! scripts, configuration files, workspace, tasks, and spec templates.

use std::path::Path;

use anyhow::Result;

use crate::constants::paths::{constructed, dir_names};
use crate::templates::harness_cli;
use crate::templates::markdown;
use crate::utils::file_writer::{ensure_dir, write_file};
use crate::utils::project_detector::{DetectedPackage, ProjectType};

// resolve_placeholders is available via super::shared if needed in future

/// Options for creating the workflow structure.
#[derive(Debug, Clone)]
pub struct WorkflowOptions {
    /// Detected or specified project type.
    pub project_type: ProjectType,
    /// Enable multi-agent pipeline with worktree support.
    pub multi_agent: bool,
    /// Skip creating local spec templates (when using remote template).
    pub skip_spec_templates: bool,
    /// Detected monorepo packages.
    pub packages: Option<Vec<DetectedPackage>>,
    /// Package names that use remote templates (skip blank spec for these).
    pub remote_spec_packages: Option<std::collections::HashSet<String>>,
}

impl Default for WorkflowOptions {
    fn default() -> Self {
        Self {
            project_type: ProjectType::Fullstack,
            multi_agent: false,
            skip_spec_templates: false,
            packages: None,
            remote_spec_packages: None,
        }
    }
}

/// Helper: sanitize a package name for use as a directory name.
fn sanitize_pkg_name(name: &str) -> String {
    // Strip leading @scope/ if present, replace non-alphanumeric with hyphens
    let stripped = if let Some(idx) = name.find('/') {
        &name[idx + 1..]
    } else {
        name
    };
    stripped
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Create workflow structure based on project type.
///
/// This creates the `.harness-cli/` directory tree by:
/// 1. Copying `scripts/` directory from embedded templates
/// 2. Writing `workflow.md` and `.gitignore`
/// 3. Creating `workspace/` with `index.md`
/// 4. Creating `tasks/` directory
/// 5. Creating `spec/` with templates (based on project type)
/// 6. Copying `worktree.yaml` if multi-agent is enabled
pub fn create_workflow_structure(cwd: &Path, options: &WorkflowOptions) -> Result<()> {
    // Create base .harness-cli directory
    ensure_dir(&cwd.join(dir_names::WORKFLOW))?;

    // Write workflow.md
    write_file(
        &cwd.join(constructed::WORKFLOW_GUIDE_FILE),
        harness_cli::workflow_md_template(),
        false,
    )?;

    // Write .gitignore
    write_file(
        &cwd.join(dir_names::WORKFLOW).join(".gitignore"),
        harness_cli::gitignore_template(),
        false,
    )?;

    // Write config.yaml
    write_file(
        &cwd.join(dir_names::WORKFLOW).join("config.yaml"),
        harness_cli::config_yaml_template(),
        false,
    )?;

    // Copy scripts individually (with executable flag)
    let scripts_base = cwd.join(constructed::SCRIPTS);
    ensure_dir(&scripts_base)?;
    for (rel_path, content) in harness_cli::get_all_scripts() {
        let target = scripts_base.join(&rel_path);
        if let Some(parent) = target.parent() {
            ensure_dir(parent)?;
        }
        let is_executable = rel_path.ends_with(".py") || rel_path.ends_with(".sh");
        write_file(&target, &content, is_executable)?;
    }

    // Create workspace/ with index.md
    let workspace_dir = cwd.join(constructed::WORKSPACE);
    ensure_dir(&workspace_dir)?;
    write_file(
        &workspace_dir.join("index.md"),
        markdown::agent_progress_index_content(),
        false,
    )?;

    // Create tasks/ directory
    ensure_dir(&cwd.join(constructed::TASKS))?;

    // Copy worktree.yaml if multi-agent enabled
    if options.multi_agent {
        write_file(
            &cwd.join(dir_names::WORKFLOW).join("worktree.yaml"),
            harness_cli::worktree_yaml_template(),
            false,
        )?;
    }

    // Create spec templates based on project type
    if let Some(ref packages) = options.packages {
        if !packages.is_empty() {
            create_spec_templates(
                cwd,
                options.project_type,
                Some(packages),
                options.remote_spec_packages.as_ref(),
            )?;
        }
    } else if !options.skip_spec_templates {
        create_spec_templates(cwd, options.project_type, None, None)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Spec template helpers
// ---------------------------------------------------------------------------

struct DocDef {
    name: &'static str,
    content: &'static str,
}

fn write_backend_docs(spec_base: &Path) -> Result<()> {
    let backend_dir = spec_base.join("backend");
    ensure_dir(&backend_dir)?;

    let docs = [
        DocDef {
            name: "index.md",
            content: markdown::backend_index_content(),
        },
        DocDef {
            name: "directory-structure.md",
            content: markdown::backend_directory_structure_content(),
        },
        DocDef {
            name: "database-guidelines.md",
            content: markdown::backend_database_guidelines_content(),
        },
        DocDef {
            name: "logging-guidelines.md",
            content: markdown::backend_logging_guidelines_content(),
        },
        DocDef {
            name: "quality-guidelines.md",
            content: markdown::backend_quality_guidelines_content(),
        },
        DocDef {
            name: "error-handling.md",
            content: markdown::backend_error_handling_content(),
        },
    ];

    for doc in &docs {
        write_file(&backend_dir.join(doc.name), doc.content, false)?;
    }
    Ok(())
}

fn write_frontend_docs(spec_base: &Path) -> Result<()> {
    let frontend_dir = spec_base.join("frontend");
    ensure_dir(&frontend_dir)?;

    let docs = [
        DocDef {
            name: "index.md",
            content: markdown::frontend_index_content(),
        },
        DocDef {
            name: "directory-structure.md",
            content: markdown::frontend_directory_structure_content(),
        },
        DocDef {
            name: "type-safety.md",
            content: markdown::frontend_type_safety_content(),
        },
        DocDef {
            name: "hook-guidelines.md",
            content: markdown::frontend_hook_guidelines_content(),
        },
        DocDef {
            name: "component-guidelines.md",
            content: markdown::frontend_component_guidelines_content(),
        },
        DocDef {
            name: "quality-guidelines.md",
            content: markdown::frontend_quality_guidelines_content(),
        },
        DocDef {
            name: "state-management.md",
            content: markdown::frontend_state_management_content(),
        },
    ];

    for doc in &docs {
        write_file(&frontend_dir.join(doc.name), doc.content, false)?;
    }
    Ok(())
}

fn write_spec_for_type(spec_base: &Path, project_type: ProjectType) -> Result<()> {
    if project_type != ProjectType::Frontend {
        write_backend_docs(spec_base)?;
    }
    if project_type != ProjectType::Backend {
        write_frontend_docs(spec_base)?;
    }
    Ok(())
}

fn create_spec_templates(
    cwd: &Path,
    project_type: ProjectType,
    packages: Option<&Vec<DetectedPackage>>,
    remote_spec_packages: Option<&std::collections::HashSet<String>>,
) -> Result<()> {
    let spec_dir = cwd.join(constructed::SPEC);
    ensure_dir(&spec_dir)?;

    // Guides -- always created
    let guides_dir = spec_dir.join("guides");
    ensure_dir(&guides_dir)?;

    let guides = [
        DocDef {
            name: "index.md",
            content: markdown::guides_index_content(),
        },
        DocDef {
            name: "cross-layer-thinking-guide.md",
            content: markdown::guides_cross_layer_thinking_guide_content(),
        },
        DocDef {
            name: "code-reuse-thinking-guide.md",
            content: markdown::guides_code_reuse_thinking_guide_content(),
        },
    ];
    for doc in &guides {
        write_file(&guides_dir.join(doc.name), doc.content, false)?;
    }

    if let Some(pkgs) = packages {
        // Monorepo mode: create spec/<name>/ for each package
        for pkg in pkgs {
            let dir_name = sanitize_pkg_name(&pkg.name);
            if let Some(remote) = remote_spec_packages {
                if remote.contains(&dir_name) {
                    continue;
                }
            }
            let pkg_spec_base = spec_dir.join(&dir_name);
            ensure_dir(&pkg_spec_base)?;
            let pkg_type = if pkg.type_ == ProjectType::Unknown {
                ProjectType::Fullstack
            } else {
                pkg.type_
            };
            write_spec_for_type(&pkg_spec_base, pkg_type)?;
        }
    } else {
        // Single-repo mode
        write_spec_for_type(&spec_dir, project_type)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_workflow_creates_dirs() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        create_workflow_structure(cwd, &WorkflowOptions::default()).unwrap();

        assert!(cwd.join(".harness-cli").is_dir());
        assert!(cwd.join(".harness-cli/workspace").is_dir());
        assert!(cwd.join(".harness-cli/tasks").is_dir());
    }

    #[test]
    fn test_create_workflow_writes_config() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        create_workflow_structure(cwd, &WorkflowOptions::default()).unwrap();

        assert!(cwd.join(".harness-cli/config.yaml").exists());
    }

    #[test]
    fn test_create_workflow_writes_gitignore() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        create_workflow_structure(cwd, &WorkflowOptions::default()).unwrap();

        assert!(cwd.join(".harness-cli/.gitignore").exists());
    }

    #[test]
    fn test_create_workflow_writes_workflow_md() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        create_workflow_structure(cwd, &WorkflowOptions::default()).unwrap();

        assert!(cwd.join(".harness-cli/workflow.md").exists());
    }

    #[test]
    fn test_create_workflow_backend_creates_backend_spec() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let options = WorkflowOptions {
            project_type: ProjectType::Backend,
            ..Default::default()
        };
        create_workflow_structure(cwd, &options).unwrap();

        assert!(cwd.join(".harness-cli/spec/backend").is_dir());
        // Backend project should NOT create frontend spec
        assert!(!cwd.join(".harness-cli/spec/frontend").exists());
    }

    #[test]
    fn test_create_workflow_frontend_creates_frontend_spec() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let options = WorkflowOptions {
            project_type: ProjectType::Frontend,
            ..Default::default()
        };
        create_workflow_structure(cwd, &options).unwrap();

        assert!(cwd.join(".harness-cli/spec/frontend").is_dir());
        // Frontend project should NOT create backend spec
        assert!(!cwd.join(".harness-cli/spec/backend").exists());
    }

    #[test]
    fn test_create_workflow_fullstack_creates_both() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let options = WorkflowOptions {
            project_type: ProjectType::Fullstack,
            ..Default::default()
        };
        create_workflow_structure(cwd, &options).unwrap();

        assert!(cwd.join(".harness-cli/spec/backend").is_dir());
        assert!(cwd.join(".harness-cli/spec/frontend").is_dir());
    }

    #[test]
    fn test_create_workflow_always_creates_guides() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let options = WorkflowOptions {
            project_type: ProjectType::Backend,
            ..Default::default()
        };
        create_workflow_structure(cwd, &options).unwrap();

        assert!(cwd.join(".harness-cli/spec/guides").is_dir());
    }

    #[test]
    fn test_create_workflow_skip_spec_templates() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let options = WorkflowOptions {
            project_type: ProjectType::Fullstack,
            skip_spec_templates: true,
            ..Default::default()
        };
        create_workflow_structure(cwd, &options).unwrap();

        // When skip_spec_templates is true and packages is None,
        // spec directory should not be created
        assert!(!cwd.join(".harness-cli/spec").exists());
    }
}
