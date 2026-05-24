use std::collections::HashMap;
use tower_lsp_server::ls_types::{
    Position, PrepareRenameResponse, Range, TextEdit, Uri, WorkspaceEdit,
};
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Handle textDocument/prepareRename request.
/// Check if the symbol at the given position can be renamed.
/// Returns the rename range if valid.
pub fn prepare_rename(
    ast: &Ast,
    semantic: &SemanticCache,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let byte_offset = crate::ast::position_to_byte_offset(ast.source, position);
    let node = ast.node_at_offset(byte_offset)?;
    let node_text = ast.node_text(node);

    // Can rename: variable names, fix IDs, compute IDs
    // Cannot rename: command names, style names, keywords
    let can_rename = semantic.variable_defs.contains_key(node_text)
        || semantic.fix_defs.contains_key(node_text)
        || semantic.compute_defs.contains_key(node_text);

    if can_rename {
        let range = crate::ast::tree_sitter_range_to_lsp(node.range(), ast.source);
        Some(PrepareRenameResponse::Range(range))
    } else {
        None
    }
}

/// Handle textDocument/rename request.
/// Rename a symbol and all its references across the workspace.
pub fn run_rename(
    ast: &Ast,
    semantic: &SemanticCache,
    position: Position,
    new_name: &str,
    uri: &str,
) -> Option<WorkspaceEdit> {
    let byte_offset = crate::ast::position_to_byte_offset(ast.source, position);
    let node = ast.node_at_offset(byte_offset)?;
    let old_name = ast.node_text(node);

    let mut changes = HashMap::new();
    let mut edits = Vec::new();

    // Add definition edit
    let def_range = crate::ast::tree_sitter_range_to_lsp(node.range(), ast.source);
    edits.push(TextEdit {
        range: def_range,
        new_text: new_name.to_string(),
    });

    // Add all reference edits
    for (ref_name, ref_loc) in &semantic.variable_refs {
        if ref_name == old_name {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: ref_loc.line,
                        character: ref_loc.character,
                    },
                    end: Position {
                        line: ref_loc.line,
                        character: ref_loc.character + old_name.len() as u32,
                    },
                },
                new_text: new_name.to_string(),
            });
        }
    }

    if edits.is_empty() {
        return None;
    }

    let uri_parsed: Uri = uri.parse().unwrap_or_else(|_| "file:///".parse().unwrap());
    changes.insert(uri_parsed, edits);

    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}
