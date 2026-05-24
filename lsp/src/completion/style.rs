use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    Documentation, MarkupContent, MarkupKind,
};
use tree_sitter::Node;
use crate::ast::Ast;
use crate::commands::COMMAND_DB;

/// Provide style-name completions when the cursor is at a position
/// where a style identifier is expected (fix_style, compute_style,
/// pair_style, bond_style, etc.).
pub fn complete_styles(ast: &Ast, offset: usize, scope: &[Node]) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Determine which style list to use based on the enclosing node context.
    let style_list = detect_style_context(ast, scope, offset);

    let Some(styles) = style_list else {
        return items;
    };

    // Extract partial text the user has already typed.
    let partial = extract_partial_style_word(ast.source, offset);

    for style in styles {
        if !partial.is_empty() && !style.name.starts_with(partial) {
            continue;
        }

        items.push(CompletionItem {
            label: style.name.clone(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(style.doc_short.clone()),
                description: style.since_version.clone(),
            }),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(build_style_signature(style)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: style.doc_full.clone(),
            })),
            sort_text: Some(format!("1_{}", style.name)),
            ..Default::default()
        });
    }

    items
}

/// Inspect the ancestor scope to decide which style list is relevant.
fn detect_style_context<'a>(
    ast: &Ast<'a>,
    scope: &[Node<'a>],
    offset: usize,
) -> Option<&'a [crate::commands::schema::StyleDef]> {
    // Walk scope from deepest to shallowest looking for hints.
    for node in scope.iter().rev() {
        match node.kind() {
            // Tree-sitter node: `fix_style` or `compute_style`
            "fix_style" => return Some(&COMMAND_DB.fix_styles),
            "compute_style" => return Some(&COMMAND_DB.compute_styles),

            // If we hit a `command` node, look at the command_name child.
            "command" => {
                // Find the command_name child.
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "command_name" {
                            let cmd_name = ast.node_text(child);
                            return style_list_for_command(cmd_name);
                        }
                    }
                }
            }

            // If we're in a `fix` node and the cursor is past fix_id,
            // we want fix styles.
            "fix" => {
                let has_style = (0..node.named_child_count())
                    .filter_map(|i| node.named_child(i))
                    .any(|c| c.kind() == "fix_style");
                if !has_style && is_past_fix_id(*node, offset) {
                    return Some(&COMMAND_DB.fix_styles);
                }
            }

            // Same for compute.
            "compute" => {
                let has_style = (0..node.named_child_count())
                    .filter_map(|i| node.named_child(i))
                    .any(|c| c.kind() == "compute_style");
                if !has_style && is_past_compute_id(*node, offset) {
                    return Some(&COMMAND_DB.compute_styles);
                }
            }

            _ => {}
        }
    }

    None
}

/// Map a command name to the appropriate style list.
fn style_list_for_command(
    name: &str,
) -> Option<&'static [crate::commands::schema::StyleDef]> {
    match name {
        "pair_style" => Some(&COMMAND_DB.pair_styles),
        "bond_style" => Some(&COMMAND_DB.bond_styles),
        "angle_style" => Some(&COMMAND_DB.angle_styles),
        "dihedral_style" => Some(&COMMAND_DB.dihedral_styles),
        "improper_style" => Some(&COMMAND_DB.improper_styles),
        "kspace_style" => Some(&COMMAND_DB.kspace_styles),
        _ => None,
    }
}

/// Check whether the byte offset is past the fix_id child of a `fix` node.
fn is_past_fix_id(node: Node, offset: usize) -> bool {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if child.kind() == "fix_id" && child.end_byte() <= offset {
                return true;
            }
        }
    }
    false
}

/// Check whether the byte offset is past the compute_id child of a `compute` node.
fn is_past_compute_id(node: Node, offset: usize) -> bool {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if child.kind() == "compute_id" && child.end_byte() <= offset {
                return true;
            }
        }
    }
    false
}

/// Extract the partial style name being typed.
fn extract_partial_style_word(source: &str, offset: usize) -> &str {
    let before = &source[..offset];
    let word_start = before
        .rfind(|c: char| c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    before[word_start..].trim_end()
}

/// Build a signature string for a style, e.g. "nvt <Tstart> <Tstop> <Tdamp>".
fn build_style_signature(style: &crate::commands::schema::StyleDef) -> String {
    let req: Vec<String> = style
        .required_args
        .iter()
        .map(|p| format!("<{}>", p.name))
        .collect();
    let opt: Vec<String> = style
        .optional_args
        .iter()
        .map(|p| format!("[{}]", p.name))
        .collect();
    let mut parts = vec![style.name.clone()];
    parts.extend(req);
    parts.extend(opt);
    parts.join(" ")
}
