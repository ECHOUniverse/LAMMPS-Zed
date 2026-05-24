use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use crate::ast::Ast;
use crate::config::DiagnosticsConfig;
use crate::commands::COMMAND_DB;

use super::make_diagnostic;

pub fn check(ast: &Ast, config: &DiagnosticsConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for cmd in ast.commands() {
        match cmd.kind {
            crate::ast::CommandKind::General => {
                let cname = ast.command_name(cmd.node).unwrap_or("");
                if cname.is_empty() {
                    continue;
                }

                // Look up in command database
                let known = COMMAND_DB.general_commands.iter()
                    .any(|c| c.name == cname);

                if !known && config.unknown_command {
                    // Try to find a similar command name (edit distance suggestion)
                    let suggestion = find_similar(cname, &COMMAND_DB.general_commands);

                    let msg = if let Some(sug) = suggestion {
                        format!(
                            "E001: 未知命令 '{}'。你是想用 '{}' 吗？如果命令来自自定义包，可以忽略此警告。",
                            cname, sug
                        )
                    } else {
                        format!(
                            "E001: 未知命令 '{}'。如果命令来自自定义包，可以忽略此警告。",
                            cname
                        )
                    };

                    diagnostics.push(make_diagnostic(
                        DiagnosticSeverity::WARNING, // Warning, not Error — user may have custom packages
                        "E001",
                        &msg,
                        cmd.node.start_byte(),
                        cmd.node.end_byte(),
                        ast.source,
                    ));
                }

                // Rough argument count check
                if config.argument_count && known {
                    if let Some(cmd_def) = COMMAND_DB.general_commands.iter()
                        .find(|c| c.name == cname)
                    {
                        let required_count = cmd_def.parameters.iter()
                            .filter(|p| p.required)
                            .count();
                        let actual_count = count_args(cmd.node, ast);

                        if actual_count < required_count {
                            diagnostics.push(make_diagnostic(
                                DiagnosticSeverity::WARNING,
                                "E004",
                                &format!(
                                    "参数不足: '{}' 需要至少 {} 个参数，但提供了 {} 个",
                                    cname, required_count, actual_count
                                ),
                                cmd.node.start_byte(),
                                cmd.node.end_byte(),
                                ast.source,
                            ));
                        }
                    }
                }
            }
            crate::ast::CommandKind::Fix => {
                // Check fix style against known styles
                let style_text = ast.node_text(cmd.node)
                    .lines()
                    .nth(0)
                    .unwrap_or("");

                // Extract the style name after fix_id and group_id
                // fix ID group style args...
                let parts: Vec<&str> = style_text.split_whitespace().collect();
                if parts.len() >= 4 {
                    let style_name = parts[3]; // fix ID group style
                    let known_style = COMMAND_DB.fix_styles.iter()
                        .any(|s| s.name == style_name);

                    if !known_style && config.unknown_command && !style_name.is_empty() {
                        diagnostics.push(make_diagnostic(
                            DiagnosticSeverity::WARNING,
                            "E001",
                            &format!("未知 fix style '{}'", style_name),
                            cmd.node.start_byte(),
                            cmd.node.end_byte(),
                            ast.source,
                        ));
                    }
                }
            }
            crate::ast::CommandKind::Compute => {
                // Check compute style against known styles
                let style_text = ast.node_text(cmd.node)
                    .lines()
                    .nth(0)
                    .unwrap_or("");
                let parts: Vec<&str> = style_text.split_whitespace().collect();
                if parts.len() >= 4 {
                    let style_name = parts[3]; // compute ID group style
                    let known_style = COMMAND_DB.compute_styles.iter()
                        .any(|s| s.name == style_name);

                    if !known_style && config.unknown_command && !style_name.is_empty() {
                        diagnostics.push(make_diagnostic(
                            DiagnosticSeverity::WARNING,
                            "E001",
                            &format!("未知 compute style '{}'", style_name),
                            cmd.node.start_byte(),
                            cmd.node.end_byte(),
                            ast.source,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    diagnostics
}

/// Count the number of arguments in a command node's args_under sections.
fn count_args(node: tree_sitter::Node, _ast: &Ast) -> usize {
    let mut count = 0;
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if child.kind() == "args_under" {
                count += child.named_child_count();
            }
        }
    }
    count
}

/// Find a similar command name using Levenshtein distance.
fn find_similar(name: &str, commands: &[crate::commands::schema::CommandDef]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    let name_lower = name.to_lowercase();

    for cmd in commands {
        let dist = levenshtein_distance(&name_lower, &cmd.name.to_lowercase());
        let threshold = std::cmp::max(name.len(), cmd.name.len()) / 3;

        if dist <= threshold {
            match best {
                Some((d, _)) if dist < d => best = Some((dist, cmd.name.as_str())),
                None => best = Some((dist, cmd.name.as_str())),
                _ => {}
            }
        }
    }

    best.map(|(_, n)| n.to_string())
}

/// Compute the Levenshtein distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr_row[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr_row[j] = (curr_row[j - 1] + 1)         // insertion
                .min(prev_row[j] + 1)                    // deletion
                .min(prev_row[j - 1] + cost);            // substitution
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}
