//! Typed diagnostics report and its Markdown rendering.
//!
//! Collection fills a [`Report`] with already-redacted facts; rendering is a
//! pure function so the format can be tested without a network.
use crate::i18n::Language;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warning,
    Failed,
    Unavailable,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// `key: value` lines.
    Fields(Vec<(String, String)>),
    Table {
        headers: Vec<&'static str>,
        rows: Vec<Vec<String>>,
    },
    /// Output of a command or a log excerpt, shown verbatim.
    Code { label: String, text: String },
    /// A hint for the reader (the AI) about how to interpret the facts.
    Note(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub status: Status,
    pub summary: String,
    pub blocks: Vec<Block>,
}

impl Section {
    pub fn new(title: &'static str, status: Status, summary: impl Into<String>) -> Self {
        Self {
            title,
            status,
            summary: summary.into(),
            blocks: Vec::new(),
        }
    }

    pub fn block(mut self, block: Block) -> Self {
        self.blocks.push(block);
        self
    }

    pub fn fields(self, fields: Vec<(&str, String)>) -> Self {
        self.block(Block::Fields(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        ))
    }

    pub fn note(self, note: impl Into<String>) -> Self {
        self.block(Block::Note(note.into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Local time with UTC offset.
    pub generated: String,
    pub window_secs: u64,
    pub sections: Vec<Section>,
}

impl Report {
    /// Markdown meant to be pasted into an AI assistant: an instruction in
    /// the user's language, a summary table, then one section per check.
    pub fn to_markdown(&self, language: Language) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Omasend network diagnostics\n");
        let _ = writeln!(out, "{}\n", language.text("diagnostics.prompt"));
        let _ = writeln!(out, "{}\n", language.text("diagnostics.problem"));
        let _ = writeln!(out, "## Summary\n");
        let _ = writeln!(out, "| # | Check | Status | Fact |");
        let _ = writeln!(out, "|---|---|---|---|");
        for (index, section) in self.sections.iter().enumerate() {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                index + 1,
                section.title,
                section.status.label(),
                cell(&section.summary)
            );
        }
        for (index, section) in self.sections.iter().enumerate() {
            let _ = writeln!(
                out,
                "\n## {}. {} — {}\n",
                index + 1,
                section.title,
                section.status.label()
            );
            let _ = writeln!(out, "{}", section.summary);
            for block in &section.blocks {
                out.push('\n');
                render_block(&mut out, block);
            }
        }
        let _ = writeln!(
            out,
            "\n---\nGenerated: {} · multicast listening window: {} s",
            self.generated, self.window_secs
        );
        out
    }
}

fn render_block(out: &mut String, block: &Block) {
    match block {
        Block::Fields(fields) => {
            for (key, value) in fields {
                let _ = writeln!(out, "- {key}: {value}");
            }
        }
        Block::Table { headers, rows } => {
            if rows.is_empty() {
                let _ = writeln!(out, "(none)");
                return;
            }
            let _ = writeln!(out, "| {} |", headers.join(" | "));
            let _ = writeln!(
                out,
                "|{}|",
                headers.iter().map(|_| "---").collect::<Vec<_>>().join("|")
            );
            for row in rows {
                let cells: Vec<String> = row.iter().map(|value| cell(value)).collect();
                let _ = writeln!(out, "| {} |", cells.join(" | "));
            }
        }
        Block::Code { label, text } => {
            let _ = writeln!(out, "{label}:");
            // A fence longer than any backtick run in the text cannot be closed early.
            let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            let fence = "`".repeat(longest.max(2) + 1);
            let body = if text.trim().is_empty() {
                "(no output)"
            } else {
                text.trim_end()
            };
            let _ = writeln!(out, "{fence}\n{body}\n{fence}");
        }
        Block::Note(note) => {
            let _ = writeln!(out, "> Note: {note}");
        }
    }
}

fn cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        Report {
            generated: "2026-11-09 16:00:00 +08:00".into(),
            window_secs: 8,
            sections: vec![
                Section::new("Basics", Status::Ok, "Omasend 0.3.1 on macos")
                    .fields(vec![("port", "53317".into())]),
                Section::new("Direct scan", Status::Warning, "no | device")
                    .block(Block::Table {
                        headers: vec!["IP", "Alias"],
                        rows: vec![vec!["10.0.0.2".into(), "a|b".into()]],
                    })
                    .block(Block::Code {
                        label: "route".into(),
                        text: "has ``` inside".into(),
                    })
                    .note("bypasses proxies"),
            ],
        }
    }

    #[test]
    fn markdown_has_prompt_summary_and_sections() {
        let markdown = sample().to_markdown(Language::En);
        assert!(markdown.starts_with("# Omasend network diagnostics\n"));
        assert!(markdown.contains(&Language::En.text("diagnostics.prompt")));
        assert!(markdown.contains("| 1 | Basics | ok | Omasend 0.3.1 on macos |"));
        assert!(markdown.contains("| 2 | Direct scan | warning | no \\| device |"));
        assert!(markdown.contains("## 2. Direct scan — warning"));
        assert!(markdown.contains("- port: 53317"));
        assert!(markdown.contains("| 10.0.0.2 | a\\|b |"));
        assert!(markdown.contains("> Note: bypasses proxies"));
        assert!(markdown.contains("Generated: 2026-11-09 16:00:00 +08:00"));
    }

    #[test]
    fn code_fence_outlasts_backticks_in_output() {
        let markdown = sample().to_markdown(Language::En);
        assert!(markdown.contains("````\nhas ``` inside\n````"));
    }

    #[test]
    fn prompt_follows_language() {
        let markdown = sample().to_markdown(Language::ZhCn);
        assert!(markdown.contains(&Language::ZhCn.text("diagnostics.prompt")));
        assert_ne!(
            Language::ZhCn.text("diagnostics.prompt"),
            Language::En.text("diagnostics.prompt")
        );
    }

    #[test]
    fn empty_table_is_explicit() {
        let mut out = String::new();
        render_block(
            &mut out,
            &Block::Table {
                headers: vec!["IP"],
                rows: vec![],
            },
        );
        assert_eq!(out, "(none)\n");
    }
}
