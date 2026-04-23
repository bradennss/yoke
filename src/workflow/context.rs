use std::path::Path;

use anyhow::{Context, Result};

use crate::template;

struct ContextBlock {
    label: String,
    source: Option<String>,
    content: String,
    tokens: usize,
}

/// Rough token estimate: ~3.5 bytes per token for mixed English/code text.
pub fn estimate_tokens(content: &str) -> usize {
    content.len().div_ceil(3)
}

pub struct ContextBuilder {
    blocks: Vec<ContextBlock>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn add_file(&mut self, label: &str, path: &Path) -> Result<&mut Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading context file {}", path.display()))?;
        let tokens = estimate_tokens(&content);
        self.blocks.push(ContextBlock {
            label: label.to_string(),
            source: Some(path.display().to_string()),
            tokens,
            content,
        });
        Ok(self)
    }

    pub fn add_content(&mut self, label: &str, content: &str) -> &mut Self {
        let tokens = estimate_tokens(content);
        self.blocks.push(ContextBlock {
            label: label.to_string(),
            source: None,
            tokens,
            content: content.to_string(),
        });
        self
    }

    pub fn total_tokens(&self) -> usize {
        self.blocks.iter().map(|b| b.tokens).sum()
    }

    /// Returns per-block metadata for logging: (label, source, tokens).
    pub fn block_stats(&self) -> Vec<(&str, Option<&str>, usize)> {
        self.blocks
            .iter()
            .map(|b| (b.label.as_str(), b.source.as_deref(), b.tokens))
            .collect()
    }

    pub fn build(&self) -> String {
        self.blocks
            .iter()
            .map(|block| {
                let source_attr = match &block.source {
                    Some(path) => format!(" source=\"{path}\""),
                    None => String::new(),
                };
                format!(
                    "<context label=\"{}\"{source_attr}>\n{}\n</context>",
                    block.label, block.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn apply(&self, template: &str) -> String {
        template::replace_vars(template, &[("context", &self.build())])
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn empty_context_produces_empty_string() {
        let builder = ContextBuilder::new();
        assert_eq!(builder.build(), "");
    }

    #[test]
    fn single_content_block() {
        let mut builder = ContextBuilder::new();
        builder.add_content("description", "Hello world");
        let result = builder.build();
        assert_eq!(
            result,
            "<context label=\"description\">\nHello world\n</context>"
        );
    }

    #[test]
    fn multiple_content_blocks() {
        let mut builder = ContextBuilder::new();
        builder.add_content("first", "aaa");
        builder.add_content("second", "bbb");
        let result = builder.build();
        assert!(result.contains("<context label=\"first\">"));
        assert!(result.contains("<context label=\"second\">"));
        assert!(result.contains("aaa"));
        assert!(result.contains("bbb"));
        let blocks: Vec<&str> = result.split("\n\n").collect();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn add_file_reads_content() {
        let dir = std::env::temp_dir().join("yoke_context_file_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("sample.txt");
        fs::write(&path, "file content here").unwrap();

        let mut builder = ContextBuilder::new();
        builder.add_file("sample", &path).unwrap();
        let result = builder.build();
        assert!(result.contains("file content here"));
        assert!(result.contains("label=\"sample\""));
        assert!(result.contains(&format!("source=\"{}\"", path.display())));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_content_has_no_source_attribute() {
        let mut builder = ContextBuilder::new();
        builder.add_content("note", "some text");
        let result = builder.build();
        assert!(!result.contains("source="));
    }

    #[test]
    fn add_file_missing_returns_error() {
        let mut builder = ContextBuilder::new();
        let result = builder.add_file("missing", Path::new("/tmp/yoke_nonexistent_file_12345.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn apply_replaces_context_placeholder() {
        let mut builder = ContextBuilder::new();
        builder.add_content("info", "some data");
        let template = "Before\n{{context}}\nAfter";
        let result = builder.apply(template);
        assert!(result.starts_with("Before\n"));
        assert!(result.ends_with("\nAfter"));
        assert!(result.contains("<context label=\"info\">"));
        assert!(result.contains("some data"));
    }

    #[test]
    fn apply_with_no_placeholder_unchanged() {
        let builder = ContextBuilder::new();
        let template = "No placeholder here";
        let result = builder.apply(template);
        assert_eq!(result, "No placeholder here");
    }

    #[test]
    fn default_creates_empty() {
        let builder = ContextBuilder::default();
        assert_eq!(builder.build(), "");
    }

    #[test]
    fn estimate_tokens_basic() {
        // 9 bytes → ceil(9/3) = 3
        assert_eq!(estimate_tokens("hello wor"), 3);
        assert_eq!(estimate_tokens(""), 0);
        // 100 bytes → ceil(100/3) = 34
        let s = "a".repeat(100);
        assert_eq!(estimate_tokens(&s), 34);
    }

    #[test]
    fn total_tokens_sums_blocks() {
        let mut builder = ContextBuilder::new();
        builder.add_content("a", "hello"); // 5 bytes → ceil(5/3) = 2
        builder.add_content("b", "world!"); // 6 bytes → ceil(6/3) = 2
        assert_eq!(builder.total_tokens(), 4);
    }

    #[test]
    fn block_stats_returns_metadata() {
        let dir = std::env::temp_dir().join("yoke_block_stats_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("sample.txt");
        fs::write(&path, "content").unwrap();

        let mut builder = ContextBuilder::new();
        builder.add_file("file_block", &path).unwrap();
        builder.add_content("inline_block", "data");

        let stats = builder.block_stats();
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].0, "file_block");
        assert!(stats[0].1.is_some());
        assert!(stats[0].2 > 0);
        assert_eq!(stats[1].0, "inline_block");
        assert!(stats[1].1.is_none());
        assert!(stats[1].2 > 0);

        let _ = fs::remove_dir_all(&dir);
    }
}
