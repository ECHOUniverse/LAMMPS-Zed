use std::collections::HashSet;
use tower_lsp_server::ls_types::{CompletionItem, CompletionItemKind};
use tree_sitter::Node;
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Provide variable-name completions.
///
/// Trigger contexts:
///   - after `$`          → all defined variable names
///   - inside `${...}`    → all defined variable names
///   - after `v_`         → variable names (equal/atom reference)
///   - after `c_`         → compute IDs
///   - after `f_`         → fix IDs
///
/// Data sources (in priority order):
///   1. SemanticCache (for cross-file references — Phase 3+)
///   2. Direct AST scan of the current document
pub fn complete_variables(
    ast: &Ast,
    semantic: &SemanticCache,
    offset: usize,
    _scope: &[Node],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let text_before = &ast.source[..offset];

    // ── Detect the trigger prefix ──────────────────────────────────

    // ${variable} — inside curly-brace expansion
    if text_before.ends_with("${") || has_unclosed_dollar_brace(text_before) {
        // Collect variable names from both sources.
        let mut names: HashSet<&str> = HashSet::new();
        for name in semantic.variable_defs.keys() {
            names.insert(name.as_str());
        }
        for name in collect_variable_defs(ast) {
            names.insert(name);
        }
        for name in names {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Variable".to_string()),
                sort_text: Some(format!("2_{}", name)),
                ..Default::default()
            });
        }
        return items;
    }

    // $variable — dollar-sign expansion (single-character or multi-char)
    if text_before.ends_with('$') {
        let mut names: HashSet<&str> = HashSet::new();
        for name in semantic.variable_defs.keys() {
            names.insert(name.as_str());
        }
        for name in collect_variable_defs(ast) {
            names.insert(name);
        }
        for name in names {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Variable ($ reference)".to_string()),
                sort_text: Some(format!("2_{}", name)),
                ..Default::default()
            });
        }
        return items;
    }

    // v_ — variable reference via underscore prefix
    if text_before.ends_with("v_") {
        let mut names: HashSet<&str> = HashSet::new();
        for name in semantic.variable_defs.keys() {
            names.insert(name.as_str());
        }
        for name in collect_variable_defs(ast) {
            names.insert(name);
        }
        for name in names {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Variable (v_ reference)".to_string()),
                insert_text: Some(format!("{} ", name)),
                sort_text: Some(format!("2_{}", name)),
                ..Default::default()
            });
        }
        return items;
    }

    // c_ — compute ID reference
    if text_before.ends_with("c_") {
        let mut names: HashSet<&str> = HashSet::new();
        for name in semantic.compute_defs.keys() {
            names.insert(name.as_str());
        }
        for name in collect_compute_defs(ast) {
            names.insert(name);
        }
        for name in names {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some("Compute (c_ reference)".to_string()),
                sort_text: Some(format!("3_{}", name)),
                ..Default::default()
            });
        }
        return items;
    }

    // f_ — fix ID reference
    if text_before.ends_with("f_") {
        let mut names: HashSet<&str> = HashSet::new();
        for name in semantic.fix_defs.keys() {
            names.insert(name.as_str());
        }
        for name in collect_fix_defs(ast) {
            names.insert(name);
        }
        for name in names {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some("Fix (f_ reference)".to_string()),
                sort_text: Some(format!("3_{}", name)),
                ..Default::default()
            });
        }
        return items;
    }

    items
}

/// Check whether we're inside an unclosed `${...}` expression,
/// e.g. the user typed `${va` and is still inside the braces.
fn has_unclosed_dollar_brace(text_before: &str) -> bool {
    if let Some(pos) = text_before.rfind("${") {
        let after = &text_before[pos..];
        // If there is no closing `}` after the `${`, we are still inside.
        !after.contains('}')
    } else {
        false
    }
}

/// Walk the AST and collect variable names from `variable_def` nodes.
fn collect_variable_defs<'a>(ast: &'a Ast<'a>) -> Vec<&'a str> {
    let mut names = Vec::new();
    collect_var_defs_recursive(ast, ast.root_node(), &mut names);
    names
}

fn collect_var_defs_recursive<'a>(ast: &Ast<'a>, node: Node<'a>, names: &mut Vec<&'a str>) {
    if node.kind() == "variable_def" {
        // Find the `variable` child that holds the name.
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "variable" {
                    names.push(ast.node_text(child));
                    break;
                }
            }
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            collect_var_defs_recursive(ast, child, names);
        }
    }
}

/// Walk the AST and collect compute IDs from `compute` nodes.
fn collect_compute_defs<'a>(ast: &'a Ast<'a>) -> Vec<&'a str> {
    let mut names = Vec::new();
    collect_comp_defs_recursive(ast, ast.root_node(), &mut names);
    names
}

fn collect_comp_defs_recursive<'a>(ast: &Ast<'a>, node: Node<'a>, names: &mut Vec<&'a str>) {
    if node.kind() == "compute" {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "compute_id" {
                    names.push(ast.node_text(child));
                    break;
                }
            }
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            collect_comp_defs_recursive(ast, child, names);
        }
    }
}

/// Walk the AST and collect fix IDs from `fix` nodes.
fn collect_fix_defs<'a>(ast: &'a Ast<'a>) -> Vec<&'a str> {
    let mut names = Vec::new();
    collect_fix_defs_recursive(ast, ast.root_node(), &mut names);
    names
}

fn collect_fix_defs_recursive<'a>(ast: &Ast<'a>, node: Node<'a>, names: &mut Vec<&'a str>) {
    if node.kind() == "fix" {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "fix_id" {
                    names.push(ast.node_text(child));
                    break;
                }
            }
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            collect_fix_defs_recursive(ast, child, names);
        }
    }
}
