//! Markdown spec templates for new projects.
//!
//! These are generic templates (`.md.txt` extension) used when scaffolding spec
//! directories for backend, frontend, and guides.

use super::extract::{get_embedded_file, MarkdownTemplates};

// ---------------------------------------------------------------------------
// Macro to reduce boilerplate for static template accessors
// ---------------------------------------------------------------------------

macro_rules! md_template {
    ($fn_name:ident, $path:expr) => {
        pub fn $fn_name() -> &'static str {
            static CONTENT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            CONTENT.get_or_init(|| {
                get_embedded_file::<MarkdownTemplates>($path).unwrap_or_default()
            })
        }
    };
}

// ---------------------------------------------------------------------------
// Root files
// ---------------------------------------------------------------------------

md_template!(agents_md_content, "agents.md");
md_template!(agent_progress_index_content, "workspace-index.md");
md_template!(gitignore_content, "gitignore.txt");
md_template!(worktree_yaml_content, "worktree.yaml.txt");

// ---------------------------------------------------------------------------
// Backend spec templates
// ---------------------------------------------------------------------------

md_template!(backend_index_content, "spec/backend/index.md.txt");
md_template!(
    backend_directory_structure_content,
    "spec/backend/directory-structure.md.txt"
);
md_template!(
    backend_database_guidelines_content,
    "spec/backend/database-guidelines.md.txt"
);
md_template!(
    backend_logging_guidelines_content,
    "spec/backend/logging-guidelines.md.txt"
);
md_template!(
    backend_quality_guidelines_content,
    "spec/backend/quality-guidelines.md.txt"
);
md_template!(
    backend_error_handling_content,
    "spec/backend/error-handling.md.txt"
);

// ---------------------------------------------------------------------------
// Frontend spec templates
// ---------------------------------------------------------------------------

md_template!(frontend_index_content, "spec/frontend/index.md.txt");
md_template!(
    frontend_directory_structure_content,
    "spec/frontend/directory-structure.md.txt"
);
md_template!(
    frontend_type_safety_content,
    "spec/frontend/type-safety.md.txt"
);
md_template!(
    frontend_hook_guidelines_content,
    "spec/frontend/hook-guidelines.md.txt"
);
md_template!(
    frontend_component_guidelines_content,
    "spec/frontend/component-guidelines.md.txt"
);
md_template!(
    frontend_quality_guidelines_content,
    "spec/frontend/quality-guidelines.md.txt"
);
md_template!(
    frontend_state_management_content,
    "spec/frontend/state-management.md.txt"
);

// ---------------------------------------------------------------------------
// Guides templates
// ---------------------------------------------------------------------------

md_template!(guides_index_content, "spec/guides/index.md.txt");
md_template!(
    guides_cross_layer_thinking_guide_content,
    "spec/guides/cross-layer-thinking-guide.md.txt"
);
md_template!(
    guides_code_reuse_thinking_guide_content,
    "spec/guides/code-reuse-thinking-guide.md.txt"
);

// ---------------------------------------------------------------------------
// KB PRD templates
// ---------------------------------------------------------------------------

md_template!(kb_prd_index_content, "kb/prd/index.md.txt");
md_template!(
    kb_prd_module_template_content,
    "kb/prd/module-template.md.txt"
);

// ---------------------------------------------------------------------------
// KB Tech templates
// ---------------------------------------------------------------------------

md_template!(kb_tech_index_content, "kb/tech/index.md.txt");
md_template!(
    kb_tech_module_template_content,
    "kb/tech/module-template.md.txt"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agents_md_non_empty() {
        let content = agents_md_content();
        assert!(!content.is_empty(), "agents.md content should be non-empty");
    }

    #[test]
    fn test_backend_templates_non_empty() {
        let templates: Vec<(&str, &str)> = vec![
            ("backend_index", backend_index_content()),
            ("backend_directory_structure", backend_directory_structure_content()),
            ("backend_database_guidelines", backend_database_guidelines_content()),
            ("backend_logging_guidelines", backend_logging_guidelines_content()),
            ("backend_quality_guidelines", backend_quality_guidelines_content()),
            ("backend_error_handling", backend_error_handling_content()),
        ];
        for (name, content) in templates {
            assert!(
                !content.is_empty(),
                "Backend template '{}' should be non-empty",
                name
            );
        }
    }

    #[test]
    fn test_frontend_templates_non_empty() {
        let templates: Vec<(&str, &str)> = vec![
            ("frontend_index", frontend_index_content()),
            ("frontend_directory_structure", frontend_directory_structure_content()),
            ("frontend_type_safety", frontend_type_safety_content()),
            ("frontend_hook_guidelines", frontend_hook_guidelines_content()),
            ("frontend_component_guidelines", frontend_component_guidelines_content()),
            ("frontend_quality_guidelines", frontend_quality_guidelines_content()),
            ("frontend_state_management", frontend_state_management_content()),
        ];
        for (name, content) in templates {
            assert!(
                !content.is_empty(),
                "Frontend template '{}' should be non-empty",
                name
            );
        }
    }

    #[test]
    fn test_guides_templates_non_empty() {
        let templates: Vec<(&str, &str)> = vec![
            ("guides_index", guides_index_content()),
            ("guides_cross_layer_thinking_guide", guides_cross_layer_thinking_guide_content()),
            ("guides_code_reuse_thinking_guide", guides_code_reuse_thinking_guide_content()),
        ];
        for (name, content) in templates {
            assert!(
                !content.is_empty(),
                "Guides template '{}' should be non-empty",
                name
            );
        }
    }

    #[test]
    fn test_agent_progress_index_non_empty() {
        let content = agent_progress_index_content();
        assert!(
            !content.is_empty(),
            "agent_progress_index (workspace-index.md) should be non-empty"
        );
    }

    #[test]
    fn test_kb_prd_templates_non_empty() {
        let templates: Vec<(&str, &str)> = vec![
            ("kb_prd_index", kb_prd_index_content()),
            ("kb_prd_module_template", kb_prd_module_template_content()),
        ];
        for (name, content) in templates {
            assert!(
                !content.is_empty(),
                "KB PRD template '{}' should be non-empty",
                name
            );
        }
    }

    #[test]
    fn test_kb_tech_templates_non_empty() {
        let templates: Vec<(&str, &str)> = vec![
            ("kb_tech_index", kb_tech_index_content()),
            ("kb_tech_module_template", kb_tech_module_template_content()),
        ];
        for (name, content) in templates {
            assert!(
                !content.is_empty(),
                "KB Tech template '{}' should be non-empty",
                name
            );
        }
    }
}
