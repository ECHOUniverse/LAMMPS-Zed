use tower_lsp_server::ls_types::{Position, Range, TextEdit};
use crate::config::FormattingConfig;

/// Handle textDocument/formatting request.
/// Performs basic formatting:
/// - Removes trailing whitespace
/// - Ensures trailing newline
pub fn run_formatting(source: &str, config: &FormattingConfig) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    let _indent = " ".repeat(config.indent_size as usize);

    let lines: Vec<&str> = source.lines().collect();
    let mut formatted = String::new();
    let mut prev_blank = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            if !prev_blank && i > 0 {
                formatted.push('\n');
                prev_blank = true;
            }
        } else {
            formatted.push_str(trimmed);
            if i < lines.len() - 1 {
                formatted.push('\n');
            }
            prev_blank = false;
        }
    }

    // Compare to source (trim trailing whitespace from source too for fair comparison)
    if formatted != source.trim_end() {
        let final_text = if formatted.ends_with('\n') {
            formatted
        } else {
            formatted + "\n"
        };

        let last_line = source.lines().count().saturating_sub(1) as u32;
        let last_char = source
            .lines()
            .last()
            .map(|l| l.len() as u32)
            .unwrap_or(0);

        edits.push(TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: last_line,
                    character: last_char,
                },
            },
            new_text: final_text,
        });
    }

    edits
}
