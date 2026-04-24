use anyhow::Result;

use crate::config::{Effort, YokeConfig};
use crate::intent::{Classification, Depth};
use crate::output::StreamDisplay;
use crate::template;

use super::invoke_sub_agent;

pub struct ClassifyResult {
    pub classification: Classification,
    pub depth: Depth,
    pub reasoning: String,
}

pub fn depth_for_classification(classification: Classification) -> Depth {
    match classification {
        Classification::Build => Depth::Full,
        Classification::Feature => Depth::Light,
        Classification::Fix => Depth::Minimal,
        Classification::Refactor => Depth::Light,
        Classification::Maintenance => Depth::Minimal,
    }
}

pub async fn classify_intent(
    config: &YokeConfig,
    description: &str,
    dry_run: bool,
) -> Result<ClassifyResult> {
    let loader = crate::prompts::PromptLoader::new(None);
    let template_str = loader.load("classify")?;

    let context = format!("## Intent description\n\n{description}");
    let prompt = template::replace_vars(&template_str, &[("context", &context)]);

    let mut display = StreamDisplay::new();

    let result = invoke_sub_agent(
        &prompt,
        &config.classify.model,
        Effort::High,
        None,
        None,
        None,
        &mut display,
        &config.retry,
        dry_run,
    )
    .await?;

    if dry_run {
        return Ok(ClassifyResult {
            classification: Classification::Feature,
            depth: Depth::Light,
            reasoning: "dry run; defaulting to feature/light".to_string(),
        });
    }

    parse_classify_response(&result.result_text)
}

fn parse_classify_response(text: &str) -> Result<ClassifyResult> {
    let start = text.find('{');
    let end = text.rfind('}');

    let (start, end) = match (start, end) {
        (Some(s), Some(e)) if s < e => (s, e),
        _ => return Ok(default_classify_result()),
    };

    let json_str = &text[start..=end];

    let value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Ok(default_classify_result()),
    };

    let classification = value
        .get("classification")
        .and_then(|v| v.as_str())
        .and_then(parse_classification_str)
        .unwrap_or(Classification::Feature);

    let depth = value
        .get("depth")
        .and_then(|v| v.as_str())
        .and_then(parse_depth_str)
        .unwrap_or_else(|| depth_for_classification(classification));

    let reasoning = value
        .get("reasoning")
        .and_then(|v| v.as_str())
        .unwrap_or("no reasoning provided")
        .to_string();

    Ok(ClassifyResult {
        classification,
        depth,
        reasoning,
    })
}

fn default_classify_result() -> ClassifyResult {
    ClassifyResult {
        classification: Classification::Feature,
        depth: Depth::Light,
        reasoning: "could not parse classification response; defaulting to feature/light"
            .to_string(),
    }
}

fn parse_classification_str(s: &str) -> Option<Classification> {
    match s.to_lowercase().as_str() {
        "build" => Some(Classification::Build),
        "feature" => Some(Classification::Feature),
        "fix" => Some(Classification::Fix),
        "refactor" => Some(Classification::Refactor),
        "maintenance" => Some(Classification::Maintenance),
        _ => None,
    }
}

fn parse_depth_str(s: &str) -> Option<Depth> {
    match s.to_lowercase().as_str() {
        "full" => Some(Depth::Full),
        "light" => Some(Depth::Light),
        "minimal" => Some(Depth::Minimal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_for_classification_build() {
        assert_eq!(depth_for_classification(Classification::Build), Depth::Full);
    }

    #[test]
    fn depth_for_classification_feature() {
        assert_eq!(
            depth_for_classification(Classification::Feature),
            Depth::Light
        );
    }

    #[test]
    fn depth_for_classification_fix() {
        assert_eq!(
            depth_for_classification(Classification::Fix),
            Depth::Minimal
        );
    }

    #[test]
    fn depth_for_classification_refactor() {
        assert_eq!(
            depth_for_classification(Classification::Refactor),
            Depth::Light
        );
    }

    #[test]
    fn depth_for_classification_maintenance() {
        assert_eq!(
            depth_for_classification(Classification::Maintenance),
            Depth::Minimal
        );
    }

    #[test]
    fn parse_valid_json_response() {
        let response =
            r#"{"classification": "fix", "depth": "minimal", "reasoning": "This is a bug fix."}"#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Fix);
        assert_eq!(result.depth, Depth::Minimal);
        assert_eq!(result.reasoning, "This is a bug fix.");
    }

    #[test]
    fn parse_json_with_surrounding_text() {
        let response = r#"Here is my analysis:
{"classification": "feature", "depth": "light", "reasoning": "Adds a new login flow."}
That's my classification."#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Feature);
        assert_eq!(result.depth, Depth::Light);
        assert_eq!(result.reasoning, "Adds a new login flow.");
    }

    #[test]
    fn parse_json_with_code_fence() {
        let response = r#"```json
{"classification": "refactor", "depth": "light", "reasoning": "Restructures auth module."}
```"#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Refactor);
        assert_eq!(result.depth, Depth::Light);
    }

    #[test]
    fn fallback_on_invalid_json() {
        let response = "This is not valid JSON at all.";
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Feature);
        assert_eq!(result.depth, Depth::Light);
    }

    #[test]
    fn fallback_on_empty_response() {
        let result = parse_classify_response("").unwrap();
        assert_eq!(result.classification, Classification::Feature);
        assert_eq!(result.depth, Depth::Light);
    }

    #[test]
    fn fallback_on_unknown_classification() {
        let response = r#"{"classification": "unknown", "depth": "light", "reasoning": "test"}"#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Feature);
        assert_eq!(result.depth, Depth::Light);
    }

    #[test]
    fn fallback_on_unknown_depth() {
        let response = r#"{"classification": "fix", "depth": "unknown", "reasoning": "test"}"#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Fix);
        assert_eq!(result.depth, Depth::Minimal);
    }

    #[test]
    fn missing_reasoning_field() {
        let response = r#"{"classification": "build", "depth": "full"}"#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Build);
        assert_eq!(result.depth, Depth::Full);
        assert_eq!(result.reasoning, "no reasoning provided");
    }

    #[test]
    fn depth_inferred_when_missing() {
        let response = r#"{"classification": "build", "reasoning": "new project"}"#;
        let result = parse_classify_response(response).unwrap();
        assert_eq!(result.classification, Classification::Build);
        assert_eq!(result.depth, Depth::Full);
    }
}
