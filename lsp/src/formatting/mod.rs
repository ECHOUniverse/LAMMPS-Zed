use std::collections::HashMap;

use tower_lsp_server::ls_types::{Position, Range, TextEdit};

use crate::ast::Ast;
use crate::config::FormattingConfig;

// ── Line classification types ──────────────────────────────

enum LineKind {
    Blank,
    Comment,
    SectionHeader,
    CommandHeader {
        keyword: String,
        keyword_width: usize,
        args_text: String,
        has_continuation: bool,
        /// Raw comment text including leading "#", e.g. "# my note"
        trailing_comment: Option<String>,
    },
    Continuation {
        text: String,
        trailing_comment: Option<String>,
    },
}

struct StmtLineInfo {
    keyword: String,
    is_header: bool,
}

// ── Public entry point ─────────────────────────────────────

pub fn run_formatting(ast: &Ast, config: &FormattingConfig) -> Vec<TextEdit> {
    let classified = classify_lines(ast);
    let alignment = compute_alignment_column(&classified, config);
    let formatted = assemble_output(ast.source, &classified, alignment, config);

    let final_text = if formatted.ends_with('\n') {
        formatted
    } else {
        formatted + "\n"
    };

    if final_text == ast.source {
        return Vec::new();
    }

    let last_line = ast.source.lines().count().saturating_sub(1) as u32;
    let last_char = ast
        .source
        .lines()
        .last()
        .map(|l| l.len() as u32)
        .unwrap_or(0);

    vec![TextEdit {
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
    }]
}

// ── Line classification ────────────────────────────────────

fn classify_lines(ast: &Ast) -> Vec<(usize, LineKind)> {
    let source = ast.source;
    let lines: Vec<&str> = source.lines().collect();

    // Build line→statement mapping from AST command nodes
    let mut line_to_stmt: HashMap<usize, StmtLineInfo> = HashMap::new();
    for cmd in ast.commands() {
        let range = cmd.node.range();
        let start_line = range.start_point.row;
        let end_line = range.end_point.row;
        let keyword = ast.command_name(cmd.node).unwrap_or("").to_string();

        for line_num in start_line..=end_line {
            line_to_stmt.insert(
                line_num,
                StmtLineInfo {
                    keyword: keyword.clone(),
                    is_header: line_num == start_line,
                },
            );
        }
    }

    // Build line→comment mapping from AST comment nodes
    let mut line_to_comment: HashMap<usize, String> = HashMap::new();
    for comment_node in ast.comments() {
        let start_line = comment_node.range().start_point.row;
        let text = ast.node_text(comment_node).to_string();
        line_to_comment.insert(start_line, text);
    }

    // Classify each line
    let mut classified: Vec<(usize, LineKind)> = Vec::new();

    for (line_num, line_text) in lines.iter().enumerate() {
        let trimmed = line_text.trim();

        if trimmed.is_empty() {
            classified.push((line_num, LineKind::Blank));
            continue;
        }

        if let Some(stmt) = line_to_stmt.get(&line_num) {
            if stmt.is_header {
                let keyword = &stmt.keyword;

                // Find keyword in the raw line and extract everything after it
                let kw_pos = line_text.find(keyword.as_str()).unwrap_or(0);
                let after_kw = line_text[kw_pos + keyword.len()..].trim_start();

                // Check for trailing "&" (line continuation marker)
                let has_continuation = after_kw.ends_with('&');

                // Strip trailing "&" to get clean args
                let args_raw = if has_continuation {
                    after_kw[..after_kw.len() - 1].trim_end()
                } else {
                    after_kw.trim_end()
                };

                // Remove trailing comment if present on this line
                let trailing_comment = line_to_comment.get(&line_num).cloned();
                let args_text = if let Some(ref c) = trailing_comment {
                    remove_trailing_text(args_raw, c)
                } else {
                    args_raw.to_string()
                };

                classified.push((
                    line_num,
                    LineKind::CommandHeader {
                        keyword: keyword.clone(),
                        keyword_width: keyword.len(),
                        args_text,
                        has_continuation,
                        trailing_comment,
                    },
                ));
            } else {
                // Continuation line (within a multi-line statement)
                let trailing_comment = line_to_comment.get(&line_num).cloned();
                let text = if let Some(ref c) = trailing_comment {
                    remove_trailing_text(trimmed, c)
                } else {
                    trimmed.to_string()
                };

                classified.push((line_num, LineKind::Continuation { text, trailing_comment }));
            }
            continue;
        }

        if let Some(comment) = line_to_comment.get(&line_num) {
            classified.push((
                line_num,
                if is_section_header(comment) {
                    LineKind::SectionHeader
                } else {
                    LineKind::Comment
                },
            ));
            continue;
        }

        // Fallback: treat as blank (should not happen for valid LAMMPS)
        classified.push((line_num, LineKind::Blank));
    }

    classified
}

/// Remove a trailing substring from text, trimming the result.
fn remove_trailing_text(text: &str, suffix: &str) -> String {
    if let Some(pos) = text.rfind(suffix) {
        text[..pos].trim_end().to_string()
    } else {
        text.to_string()
    }
}

// ── Alignment ───────────────────────────────────────────────

fn compute_alignment_column(
    classified: &[(usize, LineKind)],
    config: &FormattingConfig,
) -> usize {
    let max_kw_width = classified
        .iter()
        .filter_map(|(_, kind)| match kind {
            LineKind::CommandHeader { keyword_width, .. } => Some(*keyword_width),
            _ => None,
        })
        .max()
        .unwrap_or(0);

    let indent = config.indent_size as usize;
    if indent == 0 {
        return max_kw_width + 1;
    }

    let target = max_kw_width + indent;
    // Round up to next multiple of indent_size
    let alignment = ((target + indent - 1) / indent) * indent;
    alignment.max(max_kw_width + 1)
}

// ── Comment normalization ───────────────────────────────────

fn normalize_comment(raw: &str) -> String {
    let trimmed = raw.trim();

    // Preserve section headers like "# =====" or "# -----"
    if is_section_header(trimmed) {
        return trimmed.to_string();
    }

    // Normalize: "#text" | "#  text" → "# text"
    if trimmed.starts_with('#') {
        let after_hash = trimmed[1..].trim_start();
        format!("# {}", after_hash)
    } else {
        trimmed.to_string()
    }
}

fn is_section_header(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 4
        && bytes[0] == b'#'
        && bytes[1..].iter().all(|&b| b == bytes[1])
        && (bytes[1] == b'=' || bytes[1] == b'-')
}

// ── Output assembly ────────────────────────────────────────

fn assemble_output(
    source: &str,
    classified: &[(usize, LineKind)],
    alignment: usize,
    config: &FormattingConfig,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let total_lines = lines.len();
    let indent_size = config.indent_size as usize;

    // Build a per-line classification array indexed by line number
    let mut kind_by_line: Vec<Option<&LineKind>> = vec![None; total_lines];
    for (line_num, kind) in classified {
        if *line_num < total_lines {
            kind_by_line[*line_num] = Some(kind);
        }
    }

    // Format each line, collecting into a Vec. Empty strings represent blank lines.
    let mut formatted_lines: Vec<String> = Vec::new();
    let mut prev_blank = false;

    for i in 0..total_lines {
        let kind = kind_by_line[i];

        match kind {
            None | Some(LineKind::Blank) => {
                if !prev_blank && !formatted_lines.is_empty() {
                    formatted_lines.push(String::new());
                    prev_blank = true;
                }
            }
            Some(LineKind::Comment) => {
                formatted_lines.push(normalize_comment(lines[i].trim()));
                prev_blank = false;
            }
            Some(LineKind::SectionHeader) => {
                formatted_lines.push(lines[i].trim().to_string());
                prev_blank = false;
            }
            Some(LineKind::CommandHeader {
                keyword,
                keyword_width,
                args_text,
                has_continuation,
                trailing_comment,
            }) => {
                let mut s = String::new();
                s.push_str(keyword);

                if !args_text.is_empty() || *has_continuation {
                    let padding = if *keyword_width < alignment {
                        alignment - keyword_width
                    } else {
                        1
                    };
                    s.push_str(&" ".repeat(padding));
                    s.push_str(args_text);
                }

                if *has_continuation {
                    s.push_str(" &");
                }

                if let Some(comment) = trailing_comment {
                    s.push_str("  ");
                    s.push_str(&normalize_comment(comment));
                }

                formatted_lines.push(s);
                prev_blank = false;
            }
            Some(LineKind::Continuation { text, trailing_comment }) => {
                let mut s = String::new();
                let indent = if config.align_continuations {
                    alignment
                } else {
                    indent_size
                };
                s.push_str(&" ".repeat(indent));
                s.push_str(text);

                if let Some(comment) = trailing_comment {
                    s.push_str("  ");
                    s.push_str(&normalize_comment(comment));
                }

                formatted_lines.push(s);
                prev_blank = false;
            }
        }
    }

    // Strip trailing blank lines
    while formatted_lines.last().map_or(false, |l| l.is_empty()) {
        formatted_lines.pop();
    }

    if formatted_lines.is_empty() {
        return String::new();
    }

    formatted_lines.join("\n") + "\n"
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FormattingConfig;
    use crate::parser::ParserState;

    fn format(source: &str, config: &FormattingConfig) -> String {
        let parser = ParserState::new(source);
        let ast = Ast::new(&parser.source, &parser.tree);
        let edits = run_formatting(&ast, config);
        if edits.is_empty() {
            return source.to_string();
        }
        edits[0].new_text.clone()
    }

    fn default_config() -> FormattingConfig {
        FormattingConfig::default()
    }

    // ── Existing behaviors (preserved) ──────────────────────

    #[test]
    fn test_trailing_whitespace_removal() {
        let result = format("units metal   \n", &default_config());
        let lines: Vec<&str> = result.lines().collect();
        // Every line should be free of trailing whitespace
        for line in &lines {
            assert!(!line.ends_with(' '), "line should not have trailing whitespace: {:?}", line);
        }
        assert!(result.contains("units"), "content preserved");
        assert!(result.contains("metal"), "content preserved");
    }

    #[test]
    fn test_trailing_newline() {
        let result = format("units metal", &default_config());
        assert!(result.ends_with('\n'), "should end with newline");
    }

    #[test]
    fn test_blank_line_collapsing() {
        let result = format(
            "units metal\n\n\n\nboundary p p p\n",
            &default_config(),
        );
        let blank_count = result
            .lines()
            .filter(|l| l.trim().is_empty())
            .count();
        assert!(blank_count <= 1, "multiple blank lines should be collapsed");
        assert!(result.contains("units"), "keyword 'units' should be preserved");
        assert!(result.contains("metal"), "arg 'metal' should be preserved");
        assert!(result.contains("boundary"), "keyword 'boundary' should be preserved");
        assert!(result.contains("p p p"), "args 'p p p' should be preserved");
    }

    // ── Command alignment ───────────────────────────────────

    #[test]
    fn test_command_alignment() {
        let input = "fix 1 all nve\ncompute myTemp all temp\nunits metal\n";
        let result = format(input, &default_config());

        let lines: Vec<&str> = result.lines().collect();
        // All three commands should have their first args start at the same column
        let arg_cols: Vec<usize> = lines
            .iter()
            .map(|l| {
                // Find position after the keyword
                let trimmed = l.trim_start();
                if trimmed.starts_with("fix") {
                    l.find("1").unwrap_or(0)
                } else if trimmed.starts_with("compute") {
                    l.find("myTemp").unwrap_or(0)
                } else if trimmed.starts_with("units") {
                    l.find("metal").unwrap_or(0)
                } else {
                    0
                }
            })
            .collect();

        assert_eq!(arg_cols[0], arg_cols[1], "fix and compute should align");
        assert_eq!(arg_cols[1], arg_cols[2], "compute and units should align");
    }

    #[test]
    fn test_command_alignment_idempotent() {
        let input = "fix 1 all nve\ncompute myTemp all temp\n";
        let first = format(input, &default_config());
        let second = format(&first, &default_config());
        assert_eq!(first, second, "formatting should be idempotent");
    }

    // ── Comment normalization ───────────────────────────────

    #[test]
    fn test_comment_no_space() {
        let result = format("#comment\n", &default_config());
        assert_eq!(result, "# comment\n");
    }

    #[test]
    fn test_comment_double_space() {
        let result = format("#  comment\n", &default_config());
        assert_eq!(result, "# comment\n");
    }

    #[test]
    fn test_section_header_preserved() {
        let input = "# ============================================================\n";
        let result = format(input, &default_config());
        assert_eq!(result, input, "section header should not be modified");
    }

    #[test]
    fn test_section_header_dash_preserved() {
        let input = "# ------------------------------------------------------------\n";
        let result = format(input, &default_config());
        assert_eq!(result, input, "dash section header should not be modified");
    }

    #[test]
    fn test_comment_with_content_command() {
        // Comment after a command should be normalized
        let input = "fix 1 all nve #my fix\n";
        let result = format(input, &default_config());
        assert!(result.contains("# my fix"), "comment after command should have space after #");
    }

    // ── Continuation lines ──────────────────────────────────

    #[test]
    fn test_continuation_aligned() {
        let config = default_config();
        let input = "variable long_formula equal &\n 1.0 + 2.0 * sqrt(v_ke)\n";
        let result = format(input, &config);

        let lines: Vec<&str> = result.lines().collect();
        // Header line should end with "&"
        assert!(lines[0].contains('&'), "header should have &");
        // Continuation should be indented
        let cont_line = lines[1];
        assert!(cont_line.starts_with(' '), "continuation should be indented");
        assert!(cont_line.contains("1.0"), "content preserved");
    }

    #[test]
    fn test_continuation_simple_indent() {
        let mut config = default_config();
        config.align_continuations = false;

        let input = "variable long_formula equal &\n 1.0 + 2.0 * sqrt(v_ke)\n";
        let result = format(input, &config);

        let lines: Vec<&str> = result.lines().collect();
        // Continuation should be indented by exactly indent_size
        let cont_line = lines[1];
        let leading_spaces = cont_line.len() - cont_line.trim_start().len();
        assert_eq!(
            leading_spaces,
            config.indent_size as usize,
            "continuation should be indented by indent_size"
        );
    }

    // ── String content safety ───────────────────────────────

    #[test]
    fn test_hash_in_string_untouched() {
        let input = "variable msg string \"# not a comment\"\n";
        let result = format(input, &default_config());
        assert!(
            result.contains("\"# not a comment\""),
            "hash inside string should not be treated as comment"
        );
    }

    // ── Edge cases ──────────────────────────────────────────

    #[test]
    fn test_empty_file() {
        let result = format("", &default_config());
        assert_eq!(result, "\n", "empty file should become single newline");
    }

    #[test]
    fn test_only_comments() {
        let input = "# line one\n# line two\n";
        let result = format(input, &default_config());
        // Content has 2 comment lines + trailing newline
        assert_eq!(result.lines().count(), 2, "should have 2 comment lines");
        assert_eq!(result.lines().next().unwrap(), "# line one");
        assert!(result.ends_with('\n'), "should have trailing newline");
    }

    #[test]
    fn test_only_blank_lines() {
        let result = format("\n\n\n", &default_config());
        assert_eq!(result, "\n", "file with only blank lines should become single newline");
    }

    #[test]
    fn test_bare_command() {
        // Command with no arguments
        let result = format("run\n", &default_config());
        assert!(result.contains("run"), "bare command should be preserved");
        assert!(result.ends_with('\n'), "should end with newline");
    }

    #[test]
    fn test_full_example_file() {
        // Read the example file and verify formatting is idempotent on it
        let example_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/input.in"
        );
        let source = match std::fs::read_to_string(example_path) {
            Ok(s) => s,
            Err(_) => return, // Skip if file not found
        };

        let first = format(&source, &default_config());
        let second = format(&first, &default_config());
        assert_eq!(first, second, "formatting example file should be idempotent");
    }
}
