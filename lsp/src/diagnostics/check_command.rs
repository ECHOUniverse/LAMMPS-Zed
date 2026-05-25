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
                // Check fix style against known styles using tree-sitter field
                if let Some(style_name) = ast.field_text(cmd.node, "style") {
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
                // Check compute style against known styles using tree-sitter field
                if let Some(style_name) = ast.field_text(cmd.node, "style") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Ast;
    use crate::config::DiagnosticsConfig;
    use crate::parser::ParserState;

    #[test]
    fn test_compute_pe_atom_is_recognized() {
        let source = "compute peratom all pe/atom\n";
        let parser = ParserState::new(source);
        let ast = Ast::new(&parser.source, &parser.tree);
        let config = DiagnosticsConfig::default();
        let diagnostics = check(&ast, &config);

        // Should NOT produce "unknown compute style" warning for pe/atom
        let has_pe_atom_warning = diagnostics.iter().any(|d| {
            d.message.contains("未知 compute style") && d.message.contains("pe/atom")
        });
        assert!(
            !has_pe_atom_warning,
            "pe/atom should be recognized as a valid compute style, but got warning"
        );
    }

    #[test]
    fn test_write_data_is_recognized() {
        let source = "write_data model.data\n";
        let parser = ParserState::new(source);
        let ast = Ast::new(&parser.source, &parser.tree);
        let config = DiagnosticsConfig::default();
        let diagnostics = check(&ast, &config);

        // Should NOT produce "unknown command" warning for write_data
        let has_write_data_warning = diagnostics.iter().any(|d| {
            d.message.contains("未知命令") && d.message.contains("write_data")
        });
        assert!(
            !has_write_data_warning,
            "write_data should be recognized as a valid command, but got warning"
        );

        // Should NOT produce argument count warning
        let has_arg_count_warning = diagnostics.iter().any(|d| {
            d.message.contains("参数不足") && d.message.contains("write_data")
        });
        assert!(
            !has_arg_count_warning,
            "write_data with 1 arg should satisfy required param count, but got E004"
        );
    }

    #[test]
    fn test_write_data_with_dot_filename() {
        let source = "write_data cg.date\n";
        let parser = ParserState::new(source);
        let ast = Ast::new(&parser.source, &parser.tree);
        let config = DiagnosticsConfig::default();
        let diagnostics = check(&ast, &config);

        // Filenames with dots should be parsed as a single argument
        let has_write_data_warning = diagnostics.iter().any(|d| {
            d.message.contains("write_data")
        });
        assert!(
            !has_write_data_warning,
            "write_data with dot filename should not produce warnings, but got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_unknown_command_is_flagged() {
        let source = "nonexistent_cmd arg1 arg2\n";
        let parser = ParserState::new(source);
        let ast = Ast::new(&parser.source, &parser.tree);
        let config = DiagnosticsConfig::default();
        let diagnostics = check(&ast, &config);

        // Unknown commands should still be flagged
        let has_unknown_warning = diagnostics.iter().any(|d| {
            d.message.contains("未知命令") && d.message.contains("nonexistent_cmd")
        });
        assert!(
            has_unknown_warning,
            "nonexistent_cmd should be flagged as unknown, but no warning was produced"
        );
    }
}
