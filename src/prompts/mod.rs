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
const REVIEW_COMMON: &str = include_str!("review_common.md");
const DISCOVER_PRODUCT: &str = include_str!("discover_product.md");
const DISCOVER_TECHNICAL: &str = include_str!("discover_technical.md");
const DISCOVER_REVIEW: &str = include_str!("discover_review.md");
const CLASSIFY: &str = include_str!("classify.md");
const KNOWLEDGE_UPDATE: &str = include_str!("knowledge_update.md");
const SPEC_AMEND: &str = include_str!("spec_amend.md");

pub struct PromptLoader {
    override_dir: Option<PathBuf>,
}

impl PromptLoader {
    pub fn new(override_dir: Option<PathBuf>) -> Self {
        Self { override_dir }
    }

    pub fn load(&self, name: &str) -> Result<String> {
        let content = self.load_raw(name)?;
        Ok(self.resolve_partials(&content))
    }

    fn load_raw(&self, name: &str) -> Result<String> {
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
            "review_common" => REVIEW_COMMON,
            "discover_product" => DISCOVER_PRODUCT,
            "discover_technical" => DISCOVER_TECHNICAL,
            "discover_review" => DISCOVER_REVIEW,
            "classify" => CLASSIFY,
            "knowledge_update" => KNOWLEDGE_UPDATE,
            "spec_amend" => SPEC_AMEND,
            _ => anyhow::bail!("unknown prompt template: {name}"),
        };

        Ok(content.to_string())
    }

    /// Resolve `{{partial:name}}` references by loading and inlining the named template.
    /// Uses a distinct `{{partial:...}}` syntax to avoid collisions with regular
    /// `{{variable}}` substitution.
    fn resolve_partials(&self, content: &str) -> String {
        let mut result = content.to_string();
        while let Some(start) = result.find("{{partial:") {
            let Some(end) = result[start..].find("}}") else {
                break;
            };
            let end = start + end;
            let name = &result[start + "{{partial:".len()..end];
            let replacement = self.load_raw(name).unwrap_or_default();
            result = format!("{}{replacement}{}", &result[..start], &result[end + 2..]);
        }
        result
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
        "review_common",
        "discover_product",
        "discover_technical",
        "discover_review",
        "classify",
        "knowledge_update",
        "spec_amend",
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
        "discover_product",
        "discover_technical",
        "discover_review",
        "classify",
        "spec_amend",
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

    #[test]
    fn resolve_partials_inlines_content() {
        let loader = PromptLoader::new(None);
        let input = "before\n{{partial:review_common}}\nafter";
        let result = loader.resolve_partials(input);
        assert!(result.starts_with("before\n"));
        assert!(result.ends_with("\nafter"));
        assert!(result.contains("## Continuity"));
        assert!(result.contains("## Verdict"));
        assert!(!result.contains("{{partial:"));
    }

    #[test]
    fn resolve_partials_no_partial_unchanged() {
        let loader = PromptLoader::new(None);
        let input = "no partials here, just {{context}}";
        let result = loader.resolve_partials(input);
        assert_eq!(result, input);
    }

    #[test]
    fn resolve_partials_unknown_partial_replaced_with_empty() {
        let loader = PromptLoader::new(None);
        let input = "before{{partial:nonexistent}}after";
        let result = loader.resolve_partials(input);
        assert_eq!(result, "beforeafter");
    }

    #[test]
    fn resolve_partials_unclosed_tag_left_as_is() {
        let loader = PromptLoader::new(None);
        let input = "before{{partial:review_common";
        let result = loader.resolve_partials(input);
        assert_eq!(result, input);
    }

    #[test]
    fn load_resolves_partials_in_review_prompts() {
        let loader = PromptLoader::new(None);
        for name in ["plan_review", "code_review", "spec_review"] {
            let content = loader.load(name).unwrap();
            assert!(
                !content.contains("{{partial:"),
                "{name} still contains unresolved partial reference"
            );
            assert!(
                content.contains("## Continuity"),
                "{name} missing Continuity section from review_common"
            );
            assert!(
                content.contains("## Verdict"),
                "{name} missing Verdict section from review_common"
            );
            assert!(
                content.contains("<review-summary>"),
                "{name} missing summary tags from review_common"
            );
        }
    }

    #[test]
    fn resolve_partials_with_override() {
        let dir = std::env::temp_dir().join("yoke_partial_override_test");
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("review_common.md"), "CUSTOM PARTIAL").unwrap();

        let loader = PromptLoader::new(Some(dir.clone()));
        let input = "before\n{{partial:review_common}}\nafter";
        let result = loader.resolve_partials(input);
        assert!(result.contains("CUSTOM PARTIAL"));
        assert!(!result.contains("## Continuity"));

        let _ = fs::remove_dir_all(&dir);
    }
}
