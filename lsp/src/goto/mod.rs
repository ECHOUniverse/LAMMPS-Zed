use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, Position, Range, Uri};
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Handle textDocument/definition request.
/// Jump to:
/// - variable definitions ($x, ${x}, v_x -> variable definition)
/// - fix ID definitions (f_x -> fix definition)
/// - compute ID definitions (c_x -> compute definition)
pub fn run_goto_definition(
    ast: &Ast,
    semantic: &SemanticCache,
    position: Position,
    _uri: &str,
) -> Option<GotoDefinitionResponse> {
    let byte_offset = crate::ast::position_to_byte_offset(ast.source, position);
    let node = ast.node_at_offset(byte_offset)?;
    let node_text = ast.node_text(node);
    let node_kind = node.kind();

    let loc = match node_kind {
        "variable" => semantic
            .variable_defs
            .get(node_text)
            .or_else(|| semantic.fix_defs.get(node_text))
            .or_else(|| semantic.compute_defs.get(node_text)),
        "fix_id" => semantic.fix_defs.get(node_text),
        "compute_id" => semantic.compute_defs.get(node_text),
        _ => {
            // Check prefix references: v_x -> variable, c_x -> compute, f_x -> fix
            if node_text.starts_with("v_") {
                semantic.variable_defs.get(&node_text[2..])
            } else if node_text.starts_with("c_") {
                semantic.compute_defs.get(&node_text[2..])
            } else if node_text.starts_with("f_") {
                semantic.fix_defs.get(&node_text[2..])
            } else {
                None
            }
        }
    };

    loc.map(|l| {
        let target_uri: Uri = l.uri.parse().unwrap_or_else(|_| "file:///".parse().unwrap());
        GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range: Range {
                start: Position {
                    line: l.line,
                    character: l.character,
                },
                end: Position {
                    line: l.line,
                    character: l.character + 1,
                },
            },
        })
    })
}
