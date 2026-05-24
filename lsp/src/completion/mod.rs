pub mod command;
pub mod style;
pub mod variable;
pub mod argument;

use tower_lsp_server::ls_types::{
    CompletionItem, CompletionList, CompletionResponse, Position,
};
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Run completion at a given position in a document.
///
/// This is the main entry point for `textDocument/completion` requests.
/// It classifies the cursor context and delegates to the appropriate
/// sub-module:
///
/// | Context               | Module           | sort_text prefix |
/// |-----------------------|------------------|------------------|
/// | Command name          | `command`        | `0_`             |
/// | Style name            | `style`          | `1_`             |
/// | Variable / ID ref     | `variable`       | `2_` / `3_`      |
/// | Arguments / keywords  | `argument`       | `1_`             |
pub fn run_completion(
    ast: &Ast,
    semantic: &SemanticCache,
    position: Position,
) -> Option<CompletionResponse> {
    let byte_offset = position_to_byte_offset(ast.source, position);

    // Find the node and ancestor chain at the cursor.
    // Note: `node_at_offset` always returns Some(...) as long as offset is
    // within the source span (the root node is the fallback).
    let _node = ast.node_at_offset(byte_offset)?;
    let scope = ast.scope_at_offset(byte_offset);

    let mut items: Vec<CompletionItem> = Vec::new();

    // Command-name completions.
    items.extend(command::complete_commands(ast, byte_offset, &scope));

    // Style-name completions (fix_style, compute_style, pair/bond/angle/...).
    items.extend(style::complete_styles(ast, byte_offset, &scope));

    // Variable / ID reference completions ($, ${, v_, c_, f_).
    items.extend(variable::complete_variables(ast, semantic, byte_offset, &scope));

    // Argument / parameter / keyword / expression completions.
    items.extend(argument::complete_arguments(ast, semantic, byte_offset, &scope));

    if items.is_empty() {
        return None;
    }

    Some(CompletionResponse::List(CompletionList {
        is_incomplete: false,
        items,
    }))
}

/// Convert an LSP Position (line, character) to a byte offset into
/// the source text. Handles multi-byte UTF-8 characters correctly
/// by iterating over `char_indices`.
fn position_to_byte_offset(source: &str, position: Position) -> usize {
    let mut current_line = 0u32;
    let mut current_char = 0u32;

    for (i, ch) in source.char_indices() {
        if current_line == position.line && current_char == position.character {
            return i;
        }
        if ch == '\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += 1;
        }
    }

    // Position beyond end → clamp to source length.
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_byte_offset_start() {
        assert_eq!(
            position_to_byte_offset("hello world", Position { line: 0, character: 0 }),
            0
        );
    }

    #[test]
    fn test_position_to_byte_offset_mid() {
        assert_eq!(
            position_to_byte_offset("hello world", Position { line: 0, character: 6 }),
            6
        );
    }

    #[test]
    fn test_position_to_byte_offset_newline() {
        assert_eq!(
            position_to_byte_offset("a\nb\nc", Position { line: 1, character: 0 }),
            2
        );
    }

    #[test]
    fn test_position_to_byte_offset_beyond() {
        assert_eq!(
            position_to_byte_offset("abc", Position { line: 10, character: 0 }),
            3
        );
    }
}
