//! Qoder skill templates.
//!
//! Qoder uses `skills/<name>/SKILL.md`.

use super::extract::{get_embedded_file, list_files, QoderTemplates};

/// A skill template (directory name + SKILL.md content).
#[derive(Debug, Clone)]
pub struct SkillTemplate {
    pub name: String,
    pub content: String,
}

/// Get all skill templates from `skills/<name>/SKILL.md`.
pub fn get_all_skills() -> Vec<SkillTemplate> {
    let mut skills = Vec::new();
    for path in list_files::<QoderTemplates>() {
        if path.starts_with("skills/") && path.ends_with("/SKILL.md") {
            if let Some(content) = get_embedded_file::<QoderTemplates>(&path) {
                let name = path
                    .strip_prefix("skills/")
                    .unwrap()
                    .strip_suffix("/SKILL.md")
                    .unwrap()
                    .to_string();
                skills.push(SkillTemplate { name, content });
            }
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_skills_non_empty() {
        let skills = get_all_skills();
        assert!(!skills.is_empty(), "Qoder skills should be non-empty");
    }

    #[test]
    fn test_skill_content_non_empty() {
        for skill in get_all_skills() {
            assert!(
                !skill.content.is_empty(),
                "Skill '{}' should have non-empty content",
                skill.name
            );
        }
    }
}
