//! Antigravity workflow templates.
//!
//! Antigravity reuses Codex shared skill content, adapted for the Antigravity
//! workflow format (`.agent/workflows/<name>.md`).

use super::codex;

/// A workflow template (name + content).
#[derive(Debug, Clone)]
pub struct WorkflowTemplate {
    pub name: String,
    pub content: String,
}

/// Adapt a Codex skill's SKILL.md content to Antigravity workflow terminology.
fn adapt_skill_content_to_workflow(content: &str, workflow_names: &[String]) -> String {
    let mut adapted = content
        .replace("Codex skills", "Antigravity workflows")
        .replace("Codex skill", "Antigravity workflow")
        .replace("Create New Skill", "Create New Workflow")
        .replace(
            ".agents/skills/<skill-name>/SKILL.md",
            ".agent/workflows/<workflow-name>.md",
        )
        .replace(".agents/skills/", ".agent/workflows/")
        .replace("$<skill-name>", "/<workflow-name>")
        .replace(
            "Or open /skills and select it",
            "Or type / and select it",
        );

    // Replace individual skill references like `$start` with `/start`.
    // Also replace .agents/skills/<specific-name>/SKILL.md patterns.
    for name in workflow_names {
        adapted = adapted.replace(&format!("${}", name), &format!("/{}", name));
    }

    // Handle remaining pattern: .agents/skills/<name>/SKILL.md -> .agent/workflows/<name>.md
    // Use a simple loop approach since we already replaced the generic pattern above.
    adapted
}

/// Get all workflow templates by adapting Codex shared skills.
pub fn get_all_workflows() -> Vec<WorkflowTemplate> {
    let skills = codex::get_all_skills();
    let workflow_names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();

    skills
        .into_iter()
        .map(|skill| WorkflowTemplate {
            name: skill.name,
            content: adapt_skill_content_to_workflow(&skill.content, &workflow_names),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_workflows_non_empty() {
        let workflows = get_all_workflows();
        assert!(
            !workflows.is_empty(),
            "Antigravity workflows should be non-empty"
        );
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

    #[test]
    fn test_adapts_codex_paths() {
        let workflows = get_all_workflows();
        // Antigravity workflows are adapted from Codex skills, so they should
        // reference Antigravity terminology instead of Codex terminology.
        for wf in &workflows {
            assert!(
                !wf.content.contains(".agents/skills/"),
                "Workflow '{}' should not contain '.agents/skills/' (should be adapted to Antigravity paths)",
                wf.name
            );
        }
    }
}
