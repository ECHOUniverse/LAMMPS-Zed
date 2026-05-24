use tower_lsp_server::ls_types::{Location, Position, Range, Uri};
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Handle textDocument/references request.
/// Find all references to the symbol at the given position.
pub fn run_references(
    ast: &Ast,
    semantic: &SemanticCache,
    position: Position,
    _uri: &str,
) -> Option<Vec<Location>> {
    let byte_offset = crate::ast::position_to_byte_offset(ast.source, position);
    let node = ast.node_at_offset(byte_offset)?;
    let node_text = ast.node_text(node);

    let mut locations = Vec::new();

    for (ref_name, ref_loc) in &semantic.variable_refs {
        if ref_name == node_text {
            let target_uri: Uri = ref_loc.uri.parse().ok()?;
            locations.push(Location {
                uri: target_uri,
                range: Range {
                    start: Position {
                        line: ref_loc.line,
                        character: ref_loc.character,
                    },
                    end: Position {
                        line: ref_loc.line,
                        character: ref_loc.character + ref_name.len() as u32,
                    },
                },
            });
        }
    }

    if locations.is_empty() {
        None
    } else {
        Some(locations)
    }
}
