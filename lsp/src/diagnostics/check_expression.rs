use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use tree_sitter::TreeCursor;
use crate::ast::Ast;
use crate::semantic::SemanticCache;
use crate::config::DiagnosticsConfig;

use super::make_diagnostic;

/// Known LAMMPS math functions.
const KNOWN_FUNCTIONS: &[&str] = &[
    "sqrt", "exp", "log", "ln", "abs", "sin", "cos", "tan",
    "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "erf", "erfc", "min", "max", "ceil", "floor", "round",
    "ramp", "stagger", "stride", "displace", "swiggle", "cwiggle",
    "is_active", "is_os", "extract_setting",
];

pub fn check(
    ast: &Ast,
    _semantic: &SemanticCache,
    _config: &DiagnosticsConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk the tree to find expression and binary_op nodes
    let mut cursor = ast.root_node().walk();
    collect_expression_diagnostics(&mut cursor, ast, &mut diagnostics);

    diagnostics
}

fn collect_expression_diagnostics<'a>(
    cursor: &mut TreeCursor<'a>,
    ast: &Ast<'a>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let node = cursor.node();

    match node.kind() {
        "binary_op" => {
            // Check that binary_op has both left and right operands
            let has_left = node.child_by_field_name("left").is_some();
            let has_right = node.child_by_field_name("right").is_some();

            if !has_left || !has_right {
                diagnostics.push(make_diagnostic(
                    DiagnosticSeverity::WARNING,
                    "W004",
                    "W004: 二元运算符缺少操作数",
                    node.start_byte(),
                    node.end_byte(),
                    ast.source,
                ));
            }
        }
        "unary_op" => {
            // Check that unary_op has an operand
            let operand_count = node.named_child_count();
            if operand_count == 0 {
                diagnostics.push(make_diagnostic(
                    DiagnosticSeverity::WARNING,
                    "W004",
                    "W004: 一元运算符缺少操作数",
                    node.start_byte(),
                    node.end_byte(),
                    ast.source,
                ));
            }
        }
        "func" => {
            // Check function call name is known
            if let Some(func_field) = node.child_by_field_name("function") {
                let func_name = ast.node_text(func_field).to_lowercase();
                if !KNOWN_FUNCTIONS.contains(&func_name.as_str()) {
                    // Check for close match
                    let suggestion = find_similar_function(&func_name);
                    let msg = if let Some(sug) = suggestion {
                        format!(
                            "W004: 未知函数 '{}'。你是想用 '{}' 吗？",
                            func_name, sug
                        )
                    } else {
                        format!(
                            "W004: 未知函数 '{}'。LAMMPS 内置函数名单中未找到。",
                            func_name
                        )
                    };

                    diagnostics.push(make_diagnostic(
                        DiagnosticSeverity::WARNING,
                        "W004",
                        &msg,
                        func_field.start_byte(),
                        func_field.end_byte(),
                        ast.source,
                    ));
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            collect_expression_diagnostics(cursor, ast, diagnostics);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Find similar function name using Levenshtein distance.
fn find_similar_function(name: &str) -> Option<&'static str> {
    let mut best: Option<(usize, &str)> = None;

    for &func in KNOWN_FUNCTIONS {
        let dist = levenshtein_distance(name, func);
        let threshold = std::cmp::max(name.len(), func.len()) / 3;
        if dist <= threshold {
            match best {
                Some((d, _)) if dist < d => best = Some((dist, func)),
                None => best = Some((dist, func)),
                _ => {}
            }
        }
    }

    best.map(|(_, f)| f)
}

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
            curr_row[j] = (curr_row[j - 1] + 1)
                .min(prev_row[j] + 1)
                .min(prev_row[j - 1] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}
