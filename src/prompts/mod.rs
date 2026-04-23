use std::path::PathBuf;

use anyhow::{Context, Result};

const SYSTEM: &str = include_str!("system.md");
const SPEC_PRODUCT: &str = include_str!("spec_product.md");
const SPEC_TECHNICAL: &str = include_str!("spec_technical.md");
const SPEC_REVIEW: &str = include_str!("spec_review.md");
const PLAN_GENERATE: &str = include_str!("plan_generate.md");
const PLAN_REVIEW: &str = include_str!("plan_review.md");
const RESEARCH: &str = include_str!("research.md");
const EXECUTION: &str = include_str!("execution.md");
const CODE_REVIEW: &str = include_str!("code_review.md");
const HANDOFF: &str = include_str!("handoff.md");
const SPEC_EXTRACT: &str = include_str!("spec_extract.md");
const PHASE_PLAN_GENERATE: &str = include_str!("phase_plan_generate.md");

pub struct PromptLoader {
    override_dir: Option<PathBuf>,
}

impl PromptLoader {
    pub fn new(override_dir: Option<PathBuf>) -> Self {
        Self { override_dir }
    }

    pub fn load(&self, name: &str) -> Result<String> {
        if let Some(ref dir) = self.override_dir {
            let override_path = dir.join(format!("{name}.md"));
            if override_path.exists() {
                return std::fs::read_to_string(&override_path).with_context(|| {
                    format!("reading override prompt {}", override_path.display())
                });
            }
        }

        let content = match name {
            "system" => SYSTEM,
            "spec_product" => SPEC_PRODUCT,
            "spec_technical" => SPEC_TECHNICAL,
            "spec_review" => SPEC_REVIEW,
            "plan_generate" => PLAN_GENERATE,
            "plan_review" => PLAN_REVIEW,
            "research" => RESEARCH,
            "execution" => EXECUTION,
            "code_review" => CODE_REVIEW,
            "handoff" => HANDOFF,
            "spec_extract" => SPEC_EXTRACT,
            "phase_plan_generate" => PHASE_PLAN_GENERATE,
            _ => anyhow::bail!("unknown prompt template: {name}"),
        };

        Ok(content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const ALL_TEMPLATES: &[&str] = &[
        "system",
        "spec_product",
        "spec_technical",
        "spec_review",
        "plan_generate",
        "phase_plan_generate",
        "plan_review",
        "research",
        "execution",
        "code_review",
        "handoff",
        "spec_extract",
    ];

    #[test]
    fn all_embedded_templates_load() {
        let loader = PromptLoader::new(None);
        for name in ALL_TEMPLATES {
            let result = loader.load(name);
            assert!(result.is_ok(), "failed to load template: {name}");
            let content = result.unwrap();
            assert!(!content.is_empty(), "template {name} is empty");
        }
    }

    const CONTEXT_TEMPLATES: &[&str] = &[
        "spec_product",
        "spec_technical",
        "spec_review",
        "plan_generate",
        "phase_plan_generate",
        "plan_review",
        "research",
        "execution",
        "code_review",
        "handoff",
        "spec_extract",
    ];

    #[test]
    fn embedded_templates_contain_context_placeholder() {
        let loader = PromptLoader::new(None);
        for name in CONTEXT_TEMPLATES {
            let content = loader.load(name).unwrap();
            assert!(
                content.contains("{{context}}"),
                "template {name} missing {{{{context}}}} placeholder"
            );
        }
    }

    #[test]
    fn unknown_template_returns_error() {
        let loader = PromptLoader::new(None);
        assert!(loader.load("nonexistent").is_err());
    }

    #[test]
    fn override_takes_precedence() {
        let dir = std::env::temp_dir().join("yoke_prompt_override_test");
        let _ = fs::create_dir_all(&dir);
        let override_content = "custom override template\n{{context}}";
        fs::write(dir.join("spec_product.md"), override_content).unwrap();

        let loader = PromptLoader::new(Some(dir.clone()));
        let result = loader.load("spec_product").unwrap();
        assert_eq!(result, override_content);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn override_dir_falls_back_to_embedded() {
        let dir = std::env::temp_dir().join("yoke_prompt_fallback_test");
        let _ = fs::create_dir_all(&dir);

        let loader = PromptLoader::new(Some(dir.clone()));
        let result = loader.load("execution");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("implementation engineer"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_override_dir_uses_embedded() {
        let loader = PromptLoader::new(None);
        let result = loader.load("handoff").unwrap();
        assert!(result.contains("technical writer"));
    }
}
