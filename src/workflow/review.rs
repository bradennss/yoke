use std::future::Future;
use std::path::Path;

use anyhow::Result;

use crate::config::{Effort, YokeConfig};
use crate::output::StreamDisplay;

use super::context::ContextBuilder;
use super::invoke_sub_agent;

pub struct ReviewParams<'a> {
    pub config: &'a YokeConfig,
    pub prompt_template: &'a str,
    pub model: &'a str,
    pub effort: Effort,
    pub max_iterations: u8,
    pub tools: Option<&'a str>,
    pub system_prompt: Option<&'a str>,
    pub cwd: Option<&'a Path>,
    pub dry_run: bool,
    pub prior_findings: Option<String>,
}

pub struct ReviewIterationResult {
    pub verdict: Verdict,
    pub cost_usd: f64,
    pub result_text: String,
}

pub async fn run_review_iteration<F, Fut>(
    params: &ReviewParams<'_>,
    context_builder_fn: &F,
    display: &mut StreamDisplay,
) -> Result<ReviewIterationResult>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<ContextBuilder>>,
{
    let mut context = context_builder_fn().await?;
    if let Some(ref findings) = params.prior_findings
        && !findings.is_empty()
    {
        context.add_content("prior review findings", findings);
    }
    let prompt = context.apply(params.prompt_template);

    let result = invoke_sub_agent(
        &prompt,
        params.model,
        params.effort,
        params.tools,
        params.system_prompt,
        params.cwd,
        display,
        &params.config.retry,
        params.dry_run,
    )
    .await?;

    let verdict = parse_verdict(&result.result_text);
    Ok(ReviewIterationResult {
        verdict,
        cost_usd: result.cost_usd,
        result_text: result.result_text,
    })
}

/// Run a review loop with findings accumulation and convergence checking.
///
/// Calls `run_review_iteration` repeatedly until the verdict converges or
/// `review_params.max_iterations` is reached. Each iteration's summary is
/// accumulated and injected as context for the next iteration.
///
/// `on_iteration` is called after each iteration with `(iteration_number, cost_usd)`.
/// It is responsible for cost tracking, state step updates, and persisting state.
///
/// Returns `true` if the review converged, `false` if max iterations were reached.
pub async fn run_review_loop<F, Fut, OnIter>(
    review_params: &mut ReviewParams<'_>,
    base_effort: Effort,
    starting_iteration: u8,
    step_message_prefix: &str,
    context_fn: &F,
    mut on_iteration: OnIter,
) -> Result<bool>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<ContextBuilder>>,
    OnIter: FnMut(u8, f64) -> Result<()>,
{
    if review_params.dry_run {
        return Ok(true);
    }

    let max = review_params.max_iterations;
    let mut display = StreamDisplay::new();
    let mut accumulated_findings = String::new();

    for iteration in starting_iteration..=max {
        if iteration > 1 {
            review_params.effort = base_effort.reduced();
        }
        review_params.prior_findings = if accumulated_findings.is_empty() {
            None
        } else {
            Some(accumulated_findings.clone())
        };
        let effort_label = review_params.effort.as_str();
        crate::output::print_step(&format!(
            "{step_message_prefix}, iteration {iteration}/{max} (effort: {effort_label})"
        ));
        let iter_result = run_review_iteration(review_params, context_fn, &mut display).await?;

        let summary =
            extract_findings_summary(&iter_result.result_text, iteration, &iter_result.verdict);
        if !accumulated_findings.is_empty() {
            accumulated_findings.push_str("\n\n");
        }
        accumulated_findings.push_str(&summary);

        on_iteration(iteration, iter_result.cost_usd)?;

        if iter_result.verdict.converged() {
            return Ok(true);
        }
    }

    Ok(false)
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Clean,
    Minor,
    Changes,
}

impl Verdict {
    pub fn converged(&self) -> bool {
        matches!(self, Verdict::Clean | Verdict::Minor)
    }
}

/// Extract a verdict from the model's response text.
///
/// The prompt instructs the model to end with exactly one word on its own line:
/// `clean`, `minor`, or `changes`. This function checks the last non-empty line
/// for a standalone verdict word (stripping backticks, quotes, and punctuation).
/// If the last line is not a standalone verdict, defaults to `Changes`
/// (conservative: assume the model made edits and continue reviewing).
pub fn parse_verdict(text: &str) -> Verdict {
    let last_line = text.lines().rev().find(|l| !l.trim().is_empty());

    let Some(line) = last_line else {
        return Verdict::Changes;
    };

    let word: String = line.trim().chars().filter(|c| c.is_alphabetic()).collect();

    match word.to_lowercase().as_str() {
        "clean" => Verdict::Clean,
        "minor" => Verdict::Minor,
        "changes" => Verdict::Changes,
        _ => Verdict::Changes,
    }
}

/// Extract the structured summary block from a review iteration's response.
///
/// The review prompt instructs the model to write a `<review-summary>` block
/// containing validated areas, issues found, and fixes applied. This function
/// extracts that block and pairs it with the parsed verdict so subsequent
/// iterations have structured context about prior work.
///
/// Falls back to the last 2000 characters of the raw response if no summary
/// block is found (e.g., if the model didn't follow the format).
pub fn extract_findings_summary(result_text: &str, iteration: u8, verdict: &Verdict) -> String {
    let verdict_label = match verdict {
        Verdict::Clean => "clean",
        Verdict::Minor => "minor",
        Verdict::Changes => "changes",
    };

    let summary_content = extract_tag_content(result_text, "review-summary").unwrap_or_else(|| {
        let chars: Vec<char> = result_text.chars().collect();
        let start = chars.len().saturating_sub(2000);
        chars[start..].iter().collect()
    });

    format!("### Iteration {iteration} (verdict: {verdict_label})\n{summary_content}")
}

fn extract_tag_content(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)?;
    let end = text.find(&close)?;
    if end <= start {
        return None;
    }
    Some(text[start + open.len()..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_clean() {
        assert_eq!(
            parse_verdict("Everything looks good.\n\nclean"),
            Verdict::Clean
        );
    }

    #[test]
    fn verdict_changes() {
        assert_eq!(parse_verdict("Found issues.\n\nchanges"), Verdict::Changes);
    }

    #[test]
    fn verdict_minor() {
        assert_eq!(
            parse_verdict("Only cosmetic fixes.\n\nminor"),
            Verdict::Minor
        );
    }

    #[test]
    fn verdict_case_insensitive() {
        assert_eq!(parse_verdict("CLEAN"), Verdict::Clean);
        assert_eq!(parse_verdict("MINOR"), Verdict::Minor);
        assert_eq!(parse_verdict("CHANGES"), Verdict::Changes);
        assert_eq!(parse_verdict("Clean"), Verdict::Clean);
        assert_eq!(parse_verdict("Minor"), Verdict::Minor);
    }

    #[test]
    fn verdict_last_line_determines_verdict() {
        assert_eq!(
            parse_verdict("clean\nI made changes\n\nchanges"),
            Verdict::Changes
        );
        assert_eq!(
            parse_verdict("changes\nFixed everything\n\nclean"),
            Verdict::Clean
        );
    }

    #[test]
    fn verdict_no_match_defaults_to_changes() {
        assert_eq!(parse_verdict("no verdict word here"), Verdict::Changes);
    }

    #[test]
    fn verdict_empty_defaults_to_changes() {
        assert_eq!(parse_verdict(""), Verdict::Changes);
    }

    #[test]
    fn verdict_whole_word_only() {
        assert_eq!(parse_verdict("unclean\ncleanup"), Verdict::Changes);
    }

    #[test]
    fn verdict_with_surrounding_text() {
        let text = "I reviewed the spec and found three issues:\n\
                     1. Missing error handling for network failures.\n\
                     2. The timeout value is not configurable.\n\
                     3. No test coverage for the retry path.\n\n\
                     I fixed all three issues directly in the file.\n\n\
                     changes";
        assert_eq!(parse_verdict(text), Verdict::Changes);
    }

    #[test]
    fn verdict_clean_on_own_line() {
        let text = "All checks pass. Code quality is good.\n\nclean";
        assert_eq!(parse_verdict(text), Verdict::Clean);
    }

    #[test]
    fn verdict_backtick_wrapped() {
        assert_eq!(
            parse_verdict("Summary of fixes.\n\n`changes`"),
            Verdict::Changes
        );
        assert_eq!(parse_verdict("No issues.\n\n`clean`"), Verdict::Clean);
    }

    #[test]
    fn verdict_trailing_whitespace_and_empty_lines() {
        assert_eq!(parse_verdict("changes\n\n  \n"), Verdict::Changes);
        assert_eq!(parse_verdict("clean\n  \n\n"), Verdict::Clean);
    }

    #[test]
    fn verdict_sentence_on_last_line_defaults_to_changes() {
        assert_eq!(
            parse_verdict("Fixed all issues. The spec is now clean."),
            Verdict::Changes
        );
        assert_eq!(
            parse_verdict("I made several changes to improve clarity."),
            Verdict::Changes
        );
    }

    #[test]
    fn verdict_body_clean_ignored_when_last_line_is_changes() {
        let text = "I cleaned up the messy parts.\n\
                     The code is now clean.\n\n\
                     changes";
        assert_eq!(parse_verdict(text), Verdict::Changes);
    }

    #[test]
    fn verdict_with_period() {
        assert_eq!(parse_verdict("changes."), Verdict::Changes);
        assert_eq!(parse_verdict("clean."), Verdict::Clean);
        assert_eq!(parse_verdict("minor."), Verdict::Minor);
    }

    #[test]
    fn verdict_minor_backtick_wrapped() {
        assert_eq!(parse_verdict("Cosmetic only.\n\n`minor`"), Verdict::Minor);
    }

    #[test]
    fn verdict_converged() {
        assert!(Verdict::Clean.converged());
        assert!(Verdict::Minor.converged());
        assert!(!Verdict::Changes.converged());
    }

    #[test]
    fn extract_tag_content_basic() {
        let text = "before\n<review-summary>\nhello world\n</review-summary>\nafter";
        assert_eq!(
            extract_tag_content(text, "review-summary"),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn extract_tag_content_missing() {
        assert_eq!(extract_tag_content("no tags here", "review-summary"), None);
    }

    #[test]
    fn extract_tag_content_empty() {
        let text = "<review-summary></review-summary>";
        assert_eq!(
            extract_tag_content(text, "review-summary"),
            Some(String::new())
        );
    }

    #[test]
    fn extract_tag_content_reversed_tags() {
        let text = "</review-summary>content<review-summary>";
        assert_eq!(extract_tag_content(text, "review-summary"), None);
    }

    #[test]
    fn findings_summary_with_tag() {
        let text = "Some preamble.\n<review-summary>\nTask 1: clean\nTask 2: fixed type\n</review-summary>\n\nchanges";
        let result = extract_findings_summary(text, 3, &Verdict::Changes);
        assert!(result.starts_with("### Iteration 3 (verdict: changes)"));
        assert!(result.contains("Task 1: clean"));
        assert!(result.contains("Task 2: fixed type"));
        assert!(!result.contains("Some preamble"));
    }

    #[test]
    fn findings_summary_fallback_without_tag() {
        let text = "No summary tags here. Just raw review output.";
        let result = extract_findings_summary(text, 1, &Verdict::Minor);
        assert!(result.starts_with("### Iteration 1 (verdict: minor)"));
        assert!(result.contains("No summary tags here"));
    }

    #[test]
    fn findings_summary_fallback_truncates_long_text() {
        let long_text = "x".repeat(5000);
        let result = extract_findings_summary(&long_text, 2, &Verdict::Clean);
        assert!(result.starts_with("### Iteration 2 (verdict: clean)"));
        let content_after_header = result
            .strip_prefix("### Iteration 2 (verdict: clean)\n")
            .unwrap();
        assert_eq!(content_after_header.len(), 2000);
    }
}
