use std::future::Future;
use std::path::Path;

use anyhow::Result;

use crate::config::{Effort, YokeConfig};
use crate::output::StreamDisplay;

use super::context::ContextBuilder;
use super::invoke_sub_agent;

#[derive(Debug, PartialEq)]
pub enum ReviewOutcome {
    Clean { iterations: u8 },
    MaxIterationsReached { iterations: u8 },
}

impl ReviewOutcome {
    pub fn iterations(&self) -> u8 {
        match self {
            ReviewOutcome::Clean { iterations }
            | ReviewOutcome::MaxIterationsReached { iterations } => *iterations,
        }
    }
}

pub struct ReviewResult {
    pub outcome: ReviewOutcome,
    pub total_cost_usd: f64,
}

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
}

pub struct ReviewIterationResult {
    pub verdict: Verdict,
    pub cost_usd: f64,
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
    let context = context_builder_fn().await?;
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
    })
}

pub async fn review_cycle<F, Fut>(
    params: &ReviewParams<'_>,
    context_builder_fn: F,
    display: &mut StreamDisplay,
) -> Result<ReviewResult>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<ContextBuilder>>,
{
    if params.dry_run {
        return Ok(ReviewResult {
            outcome: ReviewOutcome::Clean { iterations: 0 },
            total_cost_usd: 0.0,
        });
    }

    let mut total_cost = 0.0;

    for iteration in 1..=params.max_iterations {
        let result = run_review_iteration(params, &context_builder_fn, display).await?;
        total_cost += result.cost_usd;

        if result.verdict == Verdict::Clean {
            return Ok(ReviewResult {
                outcome: ReviewOutcome::Clean {
                    iterations: iteration,
                },
                total_cost_usd: total_cost,
            });
        }
    }

    Ok(ReviewResult {
        outcome: ReviewOutcome::MaxIterationsReached {
            iterations: params.max_iterations,
        },
        total_cost_usd: total_cost,
    })
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Clean,
    Changes,
}

/// Extract a verdict from the model's response text.
///
/// The prompt instructs the model to end with exactly one word on its own line:
/// `clean` or `changes`. This function checks the last non-empty line for a
/// standalone verdict word (stripping backticks, quotes, and punctuation).
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
        "changes" => Verdict::Changes,
        _ => Verdict::Changes,
    }
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
    fn verdict_case_insensitive() {
        assert_eq!(parse_verdict("CLEAN"), Verdict::Clean);
        assert_eq!(parse_verdict("CHANGES"), Verdict::Changes);
        assert_eq!(parse_verdict("Clean"), Verdict::Clean);
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
    }
}
