//! Kilo CLI templates.
//!
//! Kilo uses workflow files under `workflows/*.md`.

use super::extract::{get_embedded_file, list_files, KiloTemplates};

/// A workflow template (name without extension + content).
#[derive(Debug, Clone)]
pub struct WorkflowTemplate {
    pub name: String,
    pub content: String,
}

/// Get all workflow templates from `workflows/*.md`.
pub fn get_all_workflows() -> Vec<WorkflowTemplate> {
    let mut workflows = Vec::new();
    for path in list_files::<KiloTemplates>() {
        if path.starts_with("workflows/") && path.ends_with(".md") {
            if let Some(content) = get_embedded_file::<KiloTemplates>(&path) {
                let name = path
                    .strip_prefix("workflows/")
                    .unwrap()
                    .strip_suffix(".md")
                    .unwrap()
                    .to_string();
                workflows.push(WorkflowTemplate { name, content });
            }
        }
    }
    workflows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_workflows_non_empty() {
        let workflows = get_all_workflows();
        assert!(!workflows.is_empty(), "Kilo workflows should be non-empty");
    }

    #[test]
    fn test_workflow_content_non_empty() {
        for wf in get_all_workflows() {
            assert!(
                !wf.content.is_empty(),
                "Workflow '{}' should have non-empty content",
                wf.name
            );
        }
    }
}
