use std::io::{self, Write};
use std::path::Path;

use crossterm::style::{Stylize, style};

use crate::claude::types::{ContentBlock, Delta, DeltaEvent, StreamEvent};

#[derive(Debug, PartialEq, Clone, Copy)]
enum BlockKind {
    None,
    Thinking,
    Text,
    Tool,
}

struct PendingTool {
    name: String,
    input_json: String,
}

pub struct StreamDisplay {
    in_text_block: bool,
    in_thinking_block: bool,
    last_block: BlockKind,
    total_cost: f64,
    pending_tool: Option<PendingTool>,
    cwd: Option<String>,
    context_tokens: Option<usize>,
    context_blocks: Option<usize>,
}

impl Default for StreamDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamDisplay {
    pub fn new() -> Self {
        Self {
            in_text_block: false,
            in_thinking_block: false,
            last_block: BlockKind::None,
            total_cost: 0.0,
            pending_tool: None,
            cwd: None,
            context_tokens: None,
            context_blocks: None,
        }
    }

    pub fn set_context_stats(&mut self, tokens: usize, blocks: usize) {
        self.context_tokens = Some(tokens);
        self.context_blocks = Some(blocks);
    }

    pub fn handle_event(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::System {
                subtype,
                cwd: dir,
                error,
                message,
                ..
            } => {
                if subtype == "init" {
                    if let Some(d) = dir {
                        self.cwd = Some(d.clone());
                    }
                } else if subtype == "api_retry" {
                    self.finalize_block();
                    let detail = error
                        .as_deref()
                        .or(message.as_deref())
                        .unwrap_or("unknown reason");
                    eprintln!("{}", format!("retrying API request ({detail})...").yellow());
                }
            }
            StreamEvent::StreamDelta { event: delta, .. } => self.handle_delta(delta),
            StreamEvent::Completion {
                is_error,
                result,
                total_cost_usd,
                duration_ms,
                num_turns,
                ..
            } => {
                self.finalize_block();
                if *is_error {
                    eprintln!("\n{}", style(format!("error: {result}")).red());
                }
                self.total_cost += total_cost_usd;
                let duration_secs = *duration_ms as f64 / 1000.0;
                let mut line = format!(
                    "cost: ${total_cost_usd:.4}  duration: {duration_secs:.1}s  turns: {num_turns}"
                );
                if let (Some(tokens), Some(blocks)) = (self.context_tokens, self.context_blocks) {
                    line.push_str(&format_context_stats(tokens, blocks));
                }
                if self.total_cost > *total_cost_usd {
                    line.push_str(&format!("  session total: ${:.4}", self.total_cost));
                }
                eprintln!("\n{}", style(line).dim());
            }
            _ => {}
        }
    }

    fn handle_delta(&mut self, delta: &DeltaEvent) {
        match delta {
            DeltaEvent::ContentBlockStart { content_block, .. } => match content_block {
                ContentBlock::ToolUse { name, .. } => {
                    self.finalize_block();
                    self.print_separator(BlockKind::Tool);
                    self.pending_tool = Some(PendingTool {
                        name: name.clone(),
                        input_json: String::new(),
                    });
                }
                ContentBlock::Text { .. } => {
                    self.finalize_block();
                    self.print_separator(BlockKind::Text);
                    self.in_text_block = true;
                }
                ContentBlock::Thinking { .. } => {
                    self.finalize_block();
                    self.print_separator(BlockKind::Thinking);
                    self.in_thinking_block = true;
                }
                ContentBlock::Unknown => {}
            },
            DeltaEvent::ContentBlockDelta { delta: d, .. } => match d {
                Delta::TextDelta { text } => {
                    print!("{text}");
                    let _ = io::stdout().flush();
                }
                Delta::ThinkingDelta { thinking } => {
                    eprint!("{}", style(thinking).dim());
                    let _ = io::stderr().flush();
                }
                Delta::InputJsonDelta { partial_json } => {
                    if let Some(ref mut tool) = self.pending_tool {
                        tool.input_json.push_str(partial_json);
                    }
                }
                Delta::SignatureDelta { .. } | Delta::Unknown => {}
            },
            DeltaEvent::ContentBlockStop { .. } => {
                self.finalize_block();
            }
            _ => {}
        }
    }

    fn finalize_block(&mut self) {
        if let Some(tool) = self.pending_tool.take() {
            let display_name = user_facing_name(&tool.name);
            let params = format_tool_params(&tool.name, &tool.input_json, self.cwd.as_deref());
            if params.is_empty() {
                eprintln!("{}", style(display_name).bold());
            } else {
                eprintln!("{}({params})", style(display_name).bold());
            }
            self.last_block = BlockKind::Tool;
        }
        if self.in_text_block {
            println!();
            self.in_text_block = false;
            self.last_block = BlockKind::Text;
        }
        if self.in_thinking_block {
            eprintln!();
            self.in_thinking_block = false;
            self.last_block = BlockKind::Thinking;
        }
    }

    fn print_separator(&self, next: BlockKind) {
        if self.last_block == BlockKind::None {
            return;
        }
        if self.last_block == BlockKind::Text || next == BlockKind::Text {
            eprintln!();
        }
    }
}

fn user_facing_name(tool_name: &str) -> &str {
    match tool_name {
        "Edit" => "Update",
        "Glob" => "Search",
        "Grep" => "Search",
        _ => tool_name,
    }
}

fn format_tool_params(tool_name: &str, input_json: &str, cwd: Option<&str>) -> String {
    let Ok(input) = serde_json::from_str::<serde_json::Value>(input_json) else {
        return String::new();
    };

    match tool_name {
        "Bash" => format_bash(&input),
        "Read" => format_read(&input, cwd),
        "Write" => format_file_path(&input, cwd),
        "Edit" => format_file_path(&input, cwd),
        "Glob" => format_search(&input, cwd),
        "Grep" => format_search(&input, cwd),
        "Agent" => format_agent(&input),
        "WebSearch" => str_field(&input, "query").unwrap_or_default().to_string(),
        "WebFetch" => str_field(&input, "url").unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

fn format_bash(input: &serde_json::Value) -> String {
    let Some(cmd) = str_field(input, "command") else {
        return String::new();
    };
    let first_line = cmd.lines().next().unwrap_or(cmd);
    if first_line.len() > 160 {
        let mut truncated: String = first_line.chars().take(157).collect();
        truncated.push_str("...");
        truncated
    } else if cmd.lines().count() > 1 {
        format!("{first_line} ...")
    } else {
        first_line.to_string()
    }
}

fn format_read(input: &serde_json::Value, cwd: Option<&str>) -> String {
    let Some(path) = str_field(input, "file_path") else {
        return String::new();
    };
    let display = display_path(path, cwd);
    let offset = input.get("offset").and_then(|v| v.as_u64());
    let limit = input.get("limit").and_then(|v| v.as_u64());
    let pages = str_field(input, "pages");

    if let Some(p) = pages {
        format!("{display} · pages {p}")
    } else if let (Some(off), Some(lim)) = (offset, limit) {
        format!("{display} · lines {off}-{}", off + lim)
    } else if let Some(off) = offset {
        format!("{display} · from line {off}")
    } else {
        display
    }
}

fn format_file_path(input: &serde_json::Value, cwd: Option<&str>) -> String {
    str_field(input, "file_path")
        .map(|p| display_path(p, cwd))
        .unwrap_or_default()
}

fn format_search(input: &serde_json::Value, cwd: Option<&str>) -> String {
    let Some(pattern) = str_field(input, "pattern") else {
        return String::new();
    };
    let mut result = format!("pattern: \"{pattern}\"");
    if let Some(path) = str_field(input, "path") {
        result.push_str(&format!(", path: \"{}\"", display_path(path, cwd)));
    }
    result
}

fn format_agent(input: &serde_json::Value) -> String {
    str_field(input, "description")
        .unwrap_or_default()
        .to_string()
}

fn str_field<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

pub fn print_step(label: &str) {
    eprintln!("\n{}\n", style(format!("▸ {label}")).red());
}

fn format_context_stats(tokens: usize, blocks: usize) -> String {
    if tokens >= 1000 {
        format!(
            "  context: {:.1}K tokens ({blocks} blocks)",
            tokens as f64 / 1000.0
        )
    } else {
        format!("  context: {tokens} tokens ({blocks} blocks)")
    }
}

fn display_path(path: &str, cwd: Option<&str>) -> String {
    if let Some(base) = cwd {
        Path::new(path)
            .strip_prefix(base)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.to_string())
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_facing_name_mappings() {
        assert_eq!(user_facing_name("Edit"), "Update");
        assert_eq!(user_facing_name("Glob"), "Search");
        assert_eq!(user_facing_name("Grep"), "Search");
        assert_eq!(user_facing_name("Bash"), "Bash");
        assert_eq!(user_facing_name("Read"), "Read");
        assert_eq!(user_facing_name("Write"), "Write");
    }

    #[test]
    fn format_bash_short_command() {
        let input = serde_json::json!({"command": "ls -la"});
        assert_eq!(format_bash(&input), "ls -la");
    }

    #[test]
    fn format_bash_multiline_command() {
        let input = serde_json::json!({"command": "echo hello\necho world"});
        assert_eq!(format_bash(&input), "echo hello ...");
    }

    #[test]
    fn format_bash_long_command() {
        let long = "x".repeat(200);
        let input = serde_json::json!({"command": long});
        let result = format_bash(&input);
        assert!(result.len() <= 160);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn format_read_basic() {
        let input = serde_json::json!({"file_path": "/home/user/project/src/main.rs"});
        assert_eq!(
            format_read(&input, Some("/home/user/project")),
            "src/main.rs"
        );
    }

    #[test]
    fn format_read_with_offset_and_limit() {
        let input = serde_json::json!({"file_path": "/src/main.rs", "offset": 10, "limit": 50});
        assert_eq!(format_read(&input, None), "/src/main.rs · lines 10-60");
    }

    #[test]
    fn format_read_with_pages() {
        let input = serde_json::json!({"file_path": "/doc.pdf", "pages": "1-5"});
        assert_eq!(format_read(&input, None), "/doc.pdf · pages 1-5");
    }

    #[test]
    fn format_search_with_path() {
        let input = serde_json::json!({"pattern": "*.tsx", "path": "/home/user/project/src"});
        assert_eq!(
            format_search(&input, Some("/home/user/project")),
            "pattern: \"*.tsx\", path: \"src\""
        );
    }

    #[test]
    fn format_search_without_path() {
        let input = serde_json::json!({"pattern": "TODO"});
        assert_eq!(format_search(&input, None), "pattern: \"TODO\"");
    }

    #[test]
    fn display_path_strips_cwd() {
        assert_eq!(
            display_path("/home/user/project/src/main.rs", Some("/home/user/project")),
            "src/main.rs"
        );
    }

    #[test]
    fn display_path_no_cwd() {
        assert_eq!(display_path("/full/path.rs", None), "/full/path.rs");
    }

    #[test]
    fn display_path_outside_cwd() {
        assert_eq!(
            display_path("/other/path.rs", Some("/home/user/project")),
            "/other/path.rs"
        );
    }

    #[test]
    fn format_tool_params_unknown_tool() {
        assert_eq!(format_tool_params("Unknown", "{}", None), "");
    }

    #[test]
    fn format_tool_params_invalid_json() {
        assert_eq!(format_tool_params("Bash", "not json", None), "");
    }

    #[test]
    fn separator_not_printed_at_start() {
        let display = StreamDisplay::new();
        assert_eq!(display.last_block, BlockKind::None);
    }

    #[test]
    fn format_context_stats_small() {
        assert_eq!(
            format_context_stats(500, 2),
            "  context: 500 tokens (2 blocks)"
        );
    }

    #[test]
    fn format_context_stats_large() {
        assert_eq!(
            format_context_stats(12400, 5),
            "  context: 12.4K tokens (5 blocks)"
        );
    }

    #[test]
    fn context_stats_included_in_display() {
        let mut display = StreamDisplay::new();
        display.set_context_stats(5000, 3);
        assert_eq!(display.context_tokens, Some(5000));
        assert_eq!(display.context_blocks, Some(3));
    }
}
