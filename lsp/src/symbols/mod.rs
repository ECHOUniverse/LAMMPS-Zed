use tower_lsp_server::ls_types::{DocumentSymbol, Position, Range, SymbolKind};
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Handle textDocument/documentSymbol request.
/// Build a hierarchical symbol outline from the semantic cache.
pub fn run_document_symbols(_ast: &Ast, semantic: &SemanticCache) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    // Add variable definitions
    for (name, loc) in &semantic.variable_defs {
        symbols.push(DocumentSymbol {
            name: format!("variable {}", name),
            detail: Some("Variable".to_string()),
            kind: SymbolKind::VARIABLE,
            tags: None,
            deprecated: None,
            range: default_range(loc),
            selection_range: default_range(loc),
            children: None,
        });
    }

    // Add fix definitions
    for (name, loc) in &semantic.fix_defs {
        symbols.push(DocumentSymbol {
            name: format!("fix {}", name),
            detail: Some("Fix".to_string()),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: default_range(loc),
            selection_range: default_range(loc),
            children: None,
        });
    }

    // Add compute definitions
    for (name, loc) in &semantic.compute_defs {
        symbols.push(DocumentSymbol {
            name: format!("compute {}", name),
            detail: Some("Compute".to_string()),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: default_range(loc),
            selection_range: default_range(loc),
            children: None,
        });
    }

    symbols
}

fn default_range(loc: &crate::semantic::SourceLocation) -> Range {
    Range {
        start: Position {
            line: loc.line,
            character: loc.character,
        },
        end: Position {
            line: loc.line,
            character: loc.character + 10,
        },
    }
}
